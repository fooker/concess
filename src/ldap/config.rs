use std::net::SocketAddr;
use serde::Deserialize;
use crate::ldap::dn::DN;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub base_dn: DN,

    pub listen: SocketAddr,

    // TODO: Support some kind of DN-pattern for users and groups?
}