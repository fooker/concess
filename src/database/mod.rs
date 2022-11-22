use std::path::Path;
use std::sync::Arc;

use anyhow::Context;
use anyhow::Result;
use itertools::Itertools;
use tokio::sync::RwLock;

pub use model::{Group, User};

use crate::database::data::UserEntity;
use crate::database::store::DirContainer;

mod model;
mod store;
mod data;

pub struct Database {
    users: DirContainer<UserEntity>,
}

impl Database {
    pub async fn load(path: impl AsRef<Path>) -> Result<Arc<RwLock<Self>>> {
        let users = path.as_ref().join("users");
        let users = DirContainer::load(&users).await
            .with_context(|| format!("Loading users from {:?}", &users))?;

        let database = Arc::new(RwLock::new(Self {
            users,
        }));

        return Ok(database);
    }

    pub fn users(&self) -> impl Iterator<Item=User> {
        return self.users.iter()
            .map(|user| User {
                name: &user.name,
                password: &user.password,
                first_name: &user.first_name,
                last_name: &user.last_name,
                mail: &user.mail,
                groups: &user.groups,
                database: self,
            });
    }

    pub fn groups(&self) -> impl Iterator<Item=Group> {
        return self.users.iter()
            .flat_map(|user| user.groups.iter())
            .unique()
            .map(|group| Group {
                name: &group,
                database: self,
            });
    }
}

