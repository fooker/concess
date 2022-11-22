use std::future::Future;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{Context, Error, Result};
use async_trait::async_trait;
use radius::core::code::Code;
use radius::core::packet::Packet;
use radius::core::request::Request;
use radius::core::rfc2865;
use radius::server::{RequestHandler, SecretProvider, SecretProviderError, Server};
use tokio::net::UdpSocket;
use tokio::sync::RwLock;
use tracing::warn;

use crate::Database;

pub use self::config::Config;

mod config;

impl SecretProvider for Config {
    fn fetch_secret(&self, remote_addr: SocketAddr) -> Result<Vec<u8>, SecretProviderError> {
        return Ok(self.secret.clone());
    }
}

struct Handler {
    database: Arc<RwLock<Database>>,
}

impl Handler {
    async fn handle_auth_request(&self, conn: &UdpSocket, request: &Packet) -> Result<Packet> {
        let username = rfc2865::lookup_user_name(request);
        let password = rfc2865::lookup_user_password(request);

        if let (Some(Ok(username)), Some(Ok(password))) = (username, password) {
            let database = self.database.read().await;
            let user = database.users()
                .find(|user| user.name == username)
                .filter(|user| user.verify_password(&password));
            if user.is_some() {
                return Ok(request.make_response_packet(Code::AccessAccept));
            }
        }

        return Ok(request.make_response_packet(Code::AccessReject));
    }
}

#[async_trait]
impl RequestHandler<(), Error> for Handler {
    async fn handle_radius_request(&self, conn: &UdpSocket, request: &Request) -> Result<(), Error> {
        let packet = request.get_packet();

        let response = match packet.get_code() {
            Code::AccessRequest => self.handle_auth_request(conn, packet).await?,

            _ => {
                warn!("Unhandled packet: {:?}", packet.get_code());
                packet.make_response_packet(Code::Invalid)
            },
        };

        conn.send_to(&response.encode()?, request.get_remote_addr()).await?;
        return Ok(());
    }
}

pub async fn serve(config: Config,
                   database: Arc<RwLock<Database>>,
                   shutdown: impl Future) -> Result<()> {
    let mut server = Server::listen(&config.listen.ip().to_string(), // TODO: This is stupid
                                    config.listen.port(),
                                    Handler { database },
                                    config.clone()).await // TODO: Get rid of the clone
        .with_context(|| format!("Failed to listen: {}", config.listen))?;

    return Ok(server.run(shutdown).await?);
}