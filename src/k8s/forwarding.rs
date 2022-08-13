use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::watch;
use tracing::debug;

pub async fn process_socket(
    socket: &mut TcpStream,
    stream: &mut (impl AsyncRead + AsyncWrite + Unpin),
    mut shutdown_rx: watch::Receiver<bool>,
) -> E {
    let (mut sock_read, mut sock_write) = socket.split();

    let mut read_buffer = [0; 1024];
    let mut write_buffer = [0; 1024];

    loop {
        tokio::select! {
            n = sock_read.read(&mut read_buffer) => {
                let n = match n {
                    Ok(0) => {
                        debug!("sock_read closed");
                        return E::ClientSocketClosed
                    },
                    Ok(n) => {
                        debug!("sock_read read {} bytes", n);
                        n
                    },
                    Err(e) => {
                        debug!("ClientSocketErr error = {:?}", e);
                        return E::ClientSocketErr(e);
                    }
                };

                if let Err(e) = stream.write_all(&read_buffer[0..n]).await {
                    debug!("kube socket send error error = {:?}", e);
                    return E::KubeSocketErr(e);
                }
            },
            m = stream.read(&mut write_buffer) => {
                let m = match m {
                    Ok(0) => {
                        debug!("stream_read closed");
                        return E::KubeSocketClosed
                    },
                    Ok(n) => {
                        debug!("stream_read read {} bytes", n);
                        n
                    },
                    Err(e) => {
                        debug!("ClientSocketErr error = {:?}", e);
                        return E::KubeSocketErr(e);
                    }
                };

                if let Err(e) = sock_write.write_all(&write_buffer[0..m]).await {
                    debug!("client socket send error = {:?}", e);
                    return E::ClientSocketErr(e);
                }
            }
            result = shutdown_rx.changed() => {
                if result.is_err() {
                    return E::Exit;
                }

                if *shutdown_rx.borrow() {
                    return E::Exit
                }
            }
        }
    }
}

pub enum E {
    ClientSocketClosed,
    KubeSocketClosed,
    ClientSocketErr(std::io::Error),
    KubeSocketErr(std::io::Error),
    Exit,
}
