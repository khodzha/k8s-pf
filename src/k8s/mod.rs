use std::collections::HashSet;
use std::net::{IpAddr, SocketAddr};

use config::Config;
use forwarding::E;
use k8s_openapi::api::core::v1::Pod;
use kube::config::KubeConfigOptions;
use kube::Api;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::watch;
use tracing::{error, info, warn};

pub fn start() -> (
    std::thread::JoinHandle<anyhow::Result<()>>,
    watch::Sender<bool>,
) {
    let (tx, rx) = watch::channel(false);
    let jh = std::thread::spawn(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(do_start(rx))
    });

    (jh, tx)
}

async fn do_start(mut shutdown_rx: watch::Receiver<bool>) -> anyhow::Result<()> {
    let configs = setup_configs()?;

    let mut handles = vec![];
    for config in configs {
        let client = build_client(&config).await?;
        for addr in [
            SocketAddr::from(("::1".parse::<IpAddr>().unwrap(), config.0.local_port)),
            SocketAddr::from(("127.0.0.1".parse::<IpAddr>().unwrap(), config.0.local_port)),
        ] {
            let listener = TcpListener::bind(addr).await.map_err(|e| {
                error!("Failed to bind interface, reason = {:?}", e);
                e
            })?;

            let config = config.clone();
            let client = client.clone();
            let mut shutdown_rx = shutdown_rx.clone();

            let join_handle = tokio::task::spawn(async move {
                loop {
                    match shutdown_rx.has_changed() {
                        Err(_) => break,
                        Ok(_) => {
                            if *shutdown_rx.borrow_and_update() {
                                break;
                            }
                        }
                    }

                    let (mut socket, peer) = match listener.accept().await {
                        Ok(v) => v,
                        Err(e) => {
                            error!("Failed to accept client, reason = {:?}", e);
                            continue;
                        }
                    };

                    if let Err(e) = socket.set_nodelay(true) {
                        error!("Failed to set nodelay, reason = {:?}", e);
                        continue;
                    }

                    info!(
                        port = config.0.local_port,
                        podspec = config.podspec(),
                        ?peer,
                        "Accepted new connection"
                    );

                    let config = config.clone();
                    let client = client.clone();
                    let shutdown_rx = shutdown_rx.clone();

                    tokio::task::spawn(async move {
                        let pods: Api<Pod> = Api::namespaced(client, &config.0.namespace);

                        let mut p =
                            match pods.portforward(&config.0.pod, &[config.0.pod_port]).await {
                                Ok(p) => p,
                                Err(e) => {
                                    error!("Pods error = {:?}", e);
                                    return;
                                }
                            };

                        info!(
                            pod = config.0.pod,
                            pod_port = config.0.pod_port,
                            "Portforwarder set up correctly"
                        );

                        let mut stream = p.take_stream(config.0.pod_port).unwrap();
                        match forwarding::process_socket(&mut socket, &mut stream, shutdown_rx)
                            .await
                        {
                            E::ClientSocketClosed => {}
                            E::ClientSocketErr(_) => {}
                            E::KubeSocketErr(_) | E::KubeSocketClosed => {
                                if let Err(e) = socket.shutdown().await {
                                    warn!("Failed to shutdown socket, reason = {:?}", e);
                                }
                            }
                            E::Exit => {
                                return;
                            }
                        }
                    });
                }
            });

            handles.push(join_handle);
        }
    }

    tokio::task::spawn(futures::future::join_all(handles));

    loop {
        match shutdown_rx.changed().await {
            Ok(_) => {
                if *shutdown_rx.borrow() {
                    return Ok(());
                }
            }
            // happens when sender is dropped - strange but lets exit nevertheless
            Err(_) => {
                tracing::warn!(
                    "Shutdown rx got error. Most likely sender was dropped but no message sent"
                );
                return Ok(());
            }
        }
    }
}

fn setup_configs() -> anyhow::Result<Vec<Config>> {
    let konfig = kube::config::Kubeconfig::read().map_err(|e| {
        error!("Error reading kubeconfig = {:?}", e);
        e
    })?;

    let contexts = konfig
        .contexts
        .into_iter()
        .map(|ctx| ctx.name)
        .collect::<HashSet<_>>();
    let configs = config::load();

    let invalid_config = configs
        .iter()
        .find(|config| !contexts.contains(&*config.0.context));
    if let Some(cfg) = invalid_config {
        error!(
            "You specified context = '{}', which is not present in kubeconfig",
            cfg.0.context
        );
    }

    log_configs(&configs);

    Ok(configs)
}

fn log_configs(configs: &[Config]) {
    info!("Starting, available port forwards are:");

    for config in configs {
        info!(
            "- {}, local port = {}, remote port = {}",
            config.podspec(),
            config.0.local_port,
            config.0.pod_port
        )
    }
}

async fn build_client(config: &Config) -> anyhow::Result<kube::Client> {
    let opts = KubeConfigOptions {
        context: Some(config.0.context.clone()),
        cluster: None,
        user: None,
    };
    let config = kube::Config::from_kubeconfig(&opts).await.map_err(|e| {
        error!("Error& loading config = {:?}", e);
        e
    })?;
    let client = kube::Client::try_from(config).map_err(|e| {
        error!("Error creating kube client = {:?}", e);
        e
    })?;

    Ok(client)
}

mod config;
mod forwarding;
