//! Provides the connection manager the integrates bolt_client with mobc.

use async_trait::async_trait;
use bolt_client::{Client, Metadata, Stream};
use bolt_proto::message::Success;
use bolt_proto::version::{V1_0, V2_0, V3_0, V4_0, V4_1};
use bolt_proto::{Message, Value};
use mobc::Manager;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::iter::FromIterator;
use std::net::SocketAddr;
use tokio::io::BufStream;
use tokio::net::lookup_host;
use tokio::net::ToSocketAddrs;
use tokio_util::compat::*;

pub use error::Error;

mod error;

/// A Bolt connection manager, used by mobc to create and test the health of database connections.
///
/// # Examples
///
/// ```rust,no_run
/// # use bolt_proto::version::V4_1;
/// # use mobc::{Manager, Pool};
/// # use mobc_boltrs::BoltConnectionManager;
/// # use std::collections::HashMap;
/// # use std::iter::FromIterator;
///
/// # #[tokio::main]
/// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let manager = BoltConnectionManager::new(
///         "localhost:7687",
///         None,
///         [V4_1, 0, 0, 0],
///         HashMap::from_iter(vec![
///             ("user_agent", "bolt-client/X.Y.Z"),
///             ("scheme", "basic"),
///             ("principal", "username"),
///             ("credentials", "password"),
///         ]),
///     )
///     .await?;
///
///     let pool = Pool::builder().max_open(20).build(manager);
///     let client = pool.get().await?;
///
/// #   Ok(())
/// # }
/// ```
pub struct BoltConnectionManager {
    addr: SocketAddr,
    domain: Option<String>,
    preferred_versions: [u32; 4],
    metadata: HashMap<String, Value>,
}

impl BoltConnectionManager {
    /// Creates a new [`BoltConnectionManager`]. Required arguments are the address and, if
    /// applicable the domain, of the database, preferred versions, and a hash map of metadata,
    /// such as authentication credentials.
    ///
    /// [`BoltConnectionManager`]: ./struct.BoltConnectionManager.html
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # use bolt_proto::version::V4_1;
    /// # use mobc::{Manager, Pool};
    /// # use mobc_boltrs::BoltConnectionManager;
    /// # use std::collections::HashMap;
    /// # use std::iter::FromIterator;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let manager = BoltConnectionManager::new(
    ///         "localhost:7687",
    ///         None,
    ///         [V4_1, 0, 0, 0],
    ///         HashMap::from_iter(vec![
    ///             ("user_agent", "bolt-client/X.Y.Z"),
    ///             ("scheme", "basic"),
    ///             ("principal", "username"),
    ///             ("credentials", "password"),
    ///         ]),
    ///     )
    ///     .await?;
    ///
    ///     let pool = Pool::builder().max_open(20).build(manager);
    ///     let client = pool.get().await?;
    ///
    /// #   Ok(())
    /// # }
    /// ```
    pub async fn new(
        addr: impl ToSocketAddrs,
        domain: Option<String>,
        preferred_versions: [u32; 4],
        metadata: HashMap<impl Into<String>, impl Into<Value>>,
    ) -> Result<Self, Error> {
        Ok(Self {
            addr: lookup_host(addr)
                .await?
                .next()
                .ok_or(Error::InvalidAddress)?,
            domain,
            preferred_versions,
            metadata: metadata
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        })
    }
}

#[async_trait]
impl Manager for BoltConnectionManager {
    type Connection = Client<Compat<BufStream<Stream>>>;
    type Error = Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let mut client = Client::new(
            BufStream::new(Stream::connect(self.addr, self.domain.as_ref()).await?).compat(),
            &self.preferred_versions,
        )
        .await?;
        let response = match client.version() {
            V1_0 | V2_0 => {
                let mut metadata = self.metadata.clone();
                let user_agent: String = metadata
                    .remove("user_agent")
                    .ok_or_else(|| Error::InvalidMetadata {
                        metadata: "must contain a user_agent".to_string(),
                    })
                    .map(String::try_from)??;
                client.init(user_agent, Metadata::from(metadata)).await?
            }
            V3_0 | V4_0 | V4_1 => {
                client
                    .hello(Some(Metadata::from(self.metadata.clone())))
                    .await?
            }
            _ => {
                return Err(Error::InvalidClientVersion {
                    version: client.version(),
                })
            }
        };

        match response {
            Message::Success(_) => Ok(client),
            other => Err(Error::ClientInitFailed { message: other }),
        }
    }

    async fn check(&self, mut conn: Self::Connection) -> Result<Self::Connection, Self::Error> {
        let response = match conn.version() {
            V1_0 | V2_0 => conn.run("RETURN 1;".to_string(), None).await?,
            V3_0 | V4_0 | V4_1 => {
                conn.run_with_metadata(
                    "RETURN 1;".to_string(),
                    None,
                    Some(Metadata::from(self.metadata.clone())),
                )
                .await?
            }
            _ => {
                return Err(Error::InvalidClientVersion {
                    version: conn.version(),
                })
            }
        };
        Success::try_from(response)?;
        let (response, _records) = match conn.version() {
            V1_0 | V2_0 | V3_0 => conn.pull_all().await?,
            V4_0 | V4_1 => {
                let pull_meta = Metadata::from_iter(vec![("n", -1)]);
                conn.pull(Some(pull_meta)).await?
            }
            _ => {
                return Err(Error::InvalidClientVersion {
                    version: conn.version(),
                })
            }
        };
        Success::try_from(response)?;
        Ok(conn)
    }
}

#[cfg(test)]
mod tests {
    use crate::BoltConnectionManager;
    use bolt_client::Metadata;
    use bolt_proto::message::Success;
    use bolt_proto::version::{V1_0, V2_0, V3_0, V4_0, V4_1};
    use bolt_proto::Value;
    use futures_util::future::join_all;
    use mobc::{Manager, Pool};
    use std::collections::HashMap;
    use std::convert::TryFrom;
    use std::env::var;
    use std::iter::FromIterator;

    async fn get_connection_manager(
        preferred_versions: [u32; 4],
        succeed: bool,
    ) -> BoltConnectionManager {
        let credentials = if succeed {
            var("BOLT_PASS").unwrap()
        } else {
            String::from("invalid")
        };

        BoltConnectionManager::new(
            var("BOLT_ADDR").unwrap(),
            var("BOLT_DOMAIN").ok(),
            preferred_versions,
            HashMap::from_iter(vec![
                ("user_agent", "bolt-client/X.Y.Z"),
                ("scheme", "basic"),
                ("principal", &var("BOLT_USER").unwrap()),
                ("credentials", &credentials),
            ]),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn basic_pool() {
        for &bolt_version in &[V1_0, V2_0, V3_0, V4_0, V4_1] {
            let manager = get_connection_manager([bolt_version, 0, 0, 0], true).await;
            // Don't even test connection pool if server doesn't support this Bolt version
            if manager.connect().await.is_err() {
                println!(
                    "Skipping test: server doesn't support Bolt version {:#x}.",
                    bolt_version
                );
                continue;
            }
            let pool = Pool::builder().max_open(15).build(manager);

            let mut tasks = Vec::with_capacity(50);
            for i in 1..=tasks.capacity() {
                let pool = pool.clone();
                tasks.push(async move {
                    let mut client = pool.get().await.unwrap();
                    let statement = format!("RETURN {} as num;", i);
                    let version = client.version();
                    let (response, records) = match version {
                        V1_0 | V2_0 => {
                            client.run(statement, None).await.unwrap();
                            client.pull_all().await.unwrap()
                        }
                        V3_0 => {
                            client
                                .run_with_metadata(statement, None, None)
                                .await
                                .unwrap();
                            client.pull_all().await.unwrap()
                        }
                        V4_0 | V4_1 => {
                            client
                                .run_with_metadata(statement, None, None)
                                .await
                                .unwrap();
                            client
                                .pull(Some(Metadata::from_iter(vec![("n".to_string(), 1)])))
                                .await
                                .unwrap()
                        }
                        _ => panic!("Unsupported client version: {:#x}", version),
                    };
                    assert!(Success::try_from(response).is_ok());
                    assert_eq!(records[0].fields(), &[Value::from(i as i8)]);
                });
            }
            join_all(tasks).await;
        }
    }

    #[tokio::test]
    async fn invalid_init_fails() {
        let invalid_manager = get_connection_manager([V4_1, V4_0, V3_0, V2_0], false).await;
        let pool = Pool::builder().max_open(2).build(invalid_manager);
        let conn = pool.get().await;
        assert!(matches!(conn, Err(_)));
    }
}
