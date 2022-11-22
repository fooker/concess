use std::path::{Path, PathBuf};

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;

use crate::ldap;
use crate::radius;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub data: PathBuf,

    pub ldap: ldap::Config,
    pub radius: radius::Config,
}

impl Config {
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let config = tokio::fs::read(path.as_ref()).await
            .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;
        let config = serde_yaml::from_slice(&config)
            .with_context(|| format!("Failed to pares config"))?;
        return Ok(config);
    }
}