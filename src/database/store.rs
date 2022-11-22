use std::ops::Deref;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use tracing::{debug, warn};

pub struct Named<T> {
    pub name: String,

    data: T,
}

impl<T> Deref for Named<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        return &self.data;
    }
}

struct DirEntity<T>
    where
        T: DeserializeOwned,
{
    path: PathBuf,
    data: Named<T>,
}

impl<T> DirEntity<T>
    where
        T: DeserializeOwned,
{
    pub async fn load(path: impl AsRef<Path>,
                      name: String) -> Result<Self> {
        let path = path.as_ref();

        let data = tokio::fs::read(path).await
            .with_context(|| format!("Reading entity: {:?}", path))?;

        let data = serde_yaml::from_slice(&data)
            .with_context(|| format!("Parsing entity: {:?}", path))?;

        return Ok(Self {
            path: path.to_owned(),
            data: Named {
                name: name.to_owned(),
                data,
            },
        });
    }
}

pub struct DirContainer<T>
    where
        T: DeserializeOwned,
{
    path: PathBuf,
    data: Vec<DirEntity<T>>,
}

impl<T> DirContainer<T>
    where
        T: DeserializeOwned,
{
    pub async fn load(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();

        let mut dir = tokio::fs::read_dir(path).await
            .with_context(|| format!("Reading dir: {:?}", path))?;

        let mut data = Vec::new();

        while let Some(entry) = dir.next_entry().await? {
            let name = entry.file_name();
            let name = if let Some(name) = name.to_str() { name } else {
                warn!("Ignoring entity with invalid filename: {:?}", entry.path());
                continue;
            };

            if !name.ends_with(".yaml") {
                warn!("Ignoring entity with wrong extension: {:?}", entry.path());
                continue;
            }

            // Stripping .yaml file extension
            let name = &name[..name.len() - 5];
            debug!("Loading entity: {:?} as {}", entry.path(), name);

            let entity = DirEntity::load(entry.path(), name.to_owned()).await
                .with_context(|| format!("Loading entity: {:?}", entry.path()))?;

            data.push(entity);
        }

        return Ok(Self {
            path: path.to_owned(),
            data,
        });
    }

    pub fn iter(&self) -> impl Iterator<Item=&Named<T>> {
        return self.data.iter().map(|v| &v.data);
    }
}