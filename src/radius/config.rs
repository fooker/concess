use std::net::SocketAddr;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub listen: SocketAddr,

    #[serde(deserialize_with = "deserialize_secret")]
    pub secret: Vec<u8>,
}

fn deserialize_secret<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
{
    let s: String = Deserialize::deserialize(deserializer)?;
    return Ok(s.into_bytes());
}