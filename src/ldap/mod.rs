use std::future::Future;
use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{anyhow, Context, Result};
use futures::{SinkExt, StreamExt};
use ldap3_proto::{LdapCodec, LdapPartialAttribute, LdapResultCode, LdapSearchResultEntry, SearchRequest, ServerOps, SimpleBindRequest, UnbindRequest, WhoamiRequest};
use ldap3_proto::proto::LdapMsg;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::RwLock;
use tokio_util::codec::{FramedRead, FramedWrite};
use tracing::{debug, error, info, trace};

use crate::Database;
use crate::ldap::filter::Scope;

pub use self::config::Config;
use self::dn::DN;
use self::entities::Entity;
use self::filter::Filter;

mod dn;
mod filter;
mod entities;
mod config;

enum Binding {
    Unbound,
    Bound(DN),
    Anonymous,
}

struct Session {
    addr: SocketAddr,

    config: Arc<Config>,
    database: Arc<RwLock<Database>>,

    binding: Binding,
}

impl Session {
    pub async fn do_search(&mut self, req: SearchRequest) -> Result<Vec<LdapMsg>> {
        let database = self.database.read().await;

        // todo!("Process attrs");
        // todo!("Requested attrs must be present - even if empty");
        // TODO: Move error response handling to outer callee

        let scope = Scope {
            base: req.base.parse()?,
            scope: req.scope.clone(),
        };

        let filter = match Filter::try_from(&req.filter) {
            Ok(filter) => filter,
            Err(err) => return Ok(vec![req.gen_error(LdapResultCode::InvalidAttributeSyntax, err.to_string())])
        };

        fn result_entry<E: Entity>(entity: E) -> LdapSearchResultEntry {
            let attributes = E::ATTRIBUTES.iter()
                .map(|attribute| E::get(attribute)
                    .map(|getter| LdapPartialAttribute {
                        atype: attribute.to_string(),
                        vals: getter(&entity),
                    })
                    .unwrap_or_else(|| LdapPartialAttribute {
                        atype: attribute.to_string(),
                        vals: vec![],
                    }))
                .collect();

            return LdapSearchResultEntry {
                dn: entity.dn().to_string(),
                attributes,
            };
        }

        let mut results = Vec::new();

        // Search for users
        results.extend(database.users()
            .map(|user| user.with_base_dn(&self.config.base_dn))
            .filter(|entity| scope.matches(entity))
            .filter(|user| filter.evaluate(user))
            .map(result_entry)
            .map(|entry| req.gen_result_entry(entry)));

        // Search for groups
        results.extend(database.groups()
            .map(|group| group.with_base_dn(&self.config.base_dn))
            .filter(|entity| scope.matches(entity))
            .filter(|group| filter.evaluate(group))
            .map(result_entry)
            .map(|entry| req.gen_result_entry(entry)));

        results.push(req.gen_success());

        return Ok(results);
    }

    pub async fn do_bind(&mut self, req: SimpleBindRequest) -> Result<Vec<LdapMsg>> {
        debug!("Bind Request for {:?}", req.dn);

        if req.dn.is_empty() {
            debug!("Anonymous bind");
            self.binding = Binding::Anonymous;
            return Ok(vec![req.gen_success()]);
        }

        let user_dn = DN::from_str(&req.dn)?;
        trace!("Parsed User DN: {:?}", user_dn);

        let database = self.database.read().await;

        let user = database.users()
            .map(|user| user.with_base_dn(&self.config.base_dn))
            .find(|user| user.dn() == user_dn);
        let user = if let Some(user) = user { user } else {
            debug!("No user found");
            return Ok(vec![req.gen_invalid_cred()]);
        };

        if !user.verify_password(req.pw.as_bytes()) {
            debug!("Password mismatch");
            return Ok(vec![req.gen_invalid_cred()]);
        }

        self.binding = Binding::Bound(user_dn.clone());
        return Ok(vec![req.gen_success()]);
    }

    pub async fn do_unbind(&mut self, req: UnbindRequest) -> Result<Vec<LdapMsg>> {
        self.binding = Binding::Unbound;

        // No need to notify on unbind (per rfc4511)
        return Ok(vec![]);
    }

    pub async fn do_whoami(&mut self, req: WhoamiRequest) -> Result<Vec<LdapMsg>> {
        return Ok(match &self.binding {
            Binding::Unbound => vec![],
            Binding::Bound(dn) => vec![req.gen_success(&format!("dn: {}", dn))],
            Binding::Anonymous => vec![],
        });
    }
}

async fn serve_client(socket: TcpStream,
                      addr: SocketAddr,
                      config: Arc<Config>,
                      database: Arc<RwLock<Database>>) -> Result<()> {
    let (r, w) = tokio::io::split(socket);
    let mut r = FramedRead::new(r, LdapCodec);
    let mut w = FramedWrite::new(w, LdapCodec);

    let mut session = Session {
        addr,
        config,
        database,
        binding: Binding::Unbound,
    };

    // TODO: Support processing multiplexed requests in parallel by spawning into a pool
    // TODO: Send DisconnectionNotice in case of errors

    while let Some(req) = r.next().await {
        let req = req.with_context(|| format!("Invalid request form client {}", addr))?;
        let req = ServerOps::try_from(req)
            .map_err(|()| anyhow!("Failed to map server request"))
            .with_context(|| format!("Invalid server request form client {}", addr))?;

        debug!("Got request: {:?}", req);
        let responses = match req {
            ServerOps::Search(req) => session.do_search(req).await?,
            ServerOps::SimpleBind(req) => session.do_bind(req).await?,
            ServerOps::Unbind(req) => session.do_unbind(req).await?,
            ServerOps::Whoami(req) => session.do_whoami(req).await?,
        };

        for response in responses {
            debug!("Responding with {:?}", response.op);
            w.send(response).await?;
        }

        w.flush().await?;
    }

    debug!("Client disconnected {}", addr);

    return Ok(());
}

pub async fn serve(config: Config,
                   database: Arc<RwLock<Database>>,
                   shutdown: impl Future) -> Result<()> {
    let listener = TcpListener::bind(config.listen).await
        .with_context(|| format!("Listening on {}", config.listen))?;

    let config = Arc::new(config);

    let serve = async {
        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    tokio::spawn(serve_client(socket,
                                              addr,
                                              config.clone(),
                                              database.clone()));
                }

                Err(err) => {
                    error!("Failed to accept connection: {}", err);
                }
            }
        }
    };

    tokio::select! {
         _ = shutdown => {
            info!("Server is shutting down");
            return Ok(());
        }

        res = serve => {
            return res;
        }
    }
}