use std::sync::Arc;

#[derive(Clone)]
pub struct Config(pub Arc<InnerConfig>);

#[derive(Clone)]
pub struct InnerConfig {
    pub context: String,
    pub pod: String,
    pub namespace: String,
    pub pod_port: u16,
    pub local_port: u16,
}

impl From<InnerConfig> for Config {
    fn from(config: InnerConfig) -> Self {
        Self(Arc::new(config))
    }
}

impl Config {
    pub fn podspec(&self) -> String {
        format!("{}/{}", self.0.namespace, self.0.pod)
    }
}

pub fn load() -> Vec<Config> {
    vec![
        todo!(),
    ]
}
