use password_hash::{Encoding, PasswordHashString};
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct UserEntity {
    #[serde(deserialize_with = "deserialize_password")]
    pub password: PasswordHashString,

    pub first_name: String,
    pub last_name: String,

    pub mail: String,

    pub groups: Vec<String>,
}

fn deserialize_password<'de, D>(deserializer: D) -> Result<PasswordHashString, D::Error>
    where
        D: Deserializer<'de>,
{
    let s = Deserialize::deserialize(deserializer)?;
    return PasswordHashString::parse(s, Encoding::default())
        .map_err(serde::de::Error::custom);
}