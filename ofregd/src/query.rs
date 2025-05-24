use nix::unistd::Group;
use rusqlite::{ToSql, config::DbConfig};
use serde_json::json;
use std::{
    fs, io,
    os::unix::fs::{PermissionsExt, chown},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};
use tokio_rusqlite_new::Connection;
use tracing::{error, info, warn};

use crate::store::*;
use ofreg_common::SOCK_PATH;

pub struct QuerySrv {
    db_conn: Connection,
}

impl QuerySrv {
    pub async fn new_conn() -> Self {
        let db_conn = Connection::open(DB_FILE)
            .await
            .map_err(|e| error!("db open connection error: {}", e.to_string()))
            .unwrap();

        let _ = db_conn
            .call(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode=WAL;
                PRAGMA synchronous=OFF;
                PRAGMA cache_size=-32000;
                PRAGMA busy_timeout=1000;",
                )
            })
            .await
            .map_err(|e| {
                warn!(
                    "db connection init settings error, may cause performance matter: {}",
                    e.to_string()
                )
            });

        info!("db connect and init");

        Self { db_conn }
    }

    pub async fn srv(&self) {
        fs::remove_file(SOCK_PATH).ok();
        let listener = UnixListener::bind(SOCK_PATH)
            .map_err(|_| error!("unix socket path bind error"))
            .unwrap();

        if let Ok(Some(user_group)) = Group::from_name("users") {
            let _ = chown(SOCK_PATH, None, Some(user_group.gid.into()))
                .map_err(|e| warn!("{}", e.to_string()));
            let _ = std::fs::set_permissions(SOCK_PATH, std::fs::Permissions::from_mode(0o660))
                .map_err(|e| warn!("{}", e.to_string()));
        } else {
            warn!("can't get \"users\" group info, it may cause ofreg cli can't work");
        }

        loop {
            let (mut stream, _) = listener
                .accept()
                .await
                .map_err(|e| error!("{}", e.to_string()))
                .unwrap();

            let db_conn = self.db_conn.clone();
            tokio::spawn(async move {
                let payload_len = stream
                    .read_u32()
                    .await
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();
                let mut buf = vec![0u8; payload_len as usize];
                stream
                    .read_exact(buf.as_mut_slice())
                    .await
                    .map_err(|e| error!("{}", e.to_string()))
                    .unwrap();

                //info!("read {} bytes: {}", payload_len, query_str);

                if let Ok(result) = db_conn
                    .call(move |conn| {
                        let query_str = str::from_utf8(buf.as_slice())?;
                        let mut stmt = conn.prepare(query_str)?;
                        stmt.query_map([], |row| {
                            Ok(OfregData {
                                cmd: row.get(0)?,
                                op_file: row.get(1)?,
                                time: row.get(2)?,
                            })
                        })?
                        .collect::<Result<Vec<OfregData>, rusqlite::Error>>()
                    })
                    .await
                {
                    for item in result {
                        let item_bin = json!(item).to_string();
                        stream
                            .write_u32(item_bin.len() as u32)
                            .await
                            .map_err(|e| {
                                if e.kind() != std::io::ErrorKind::BrokenPipe {
                                    e
                                }
                            })
                            .unwrap();
                        stream.write_all(item_bin.as_bytes()).await.unwrap();
                    }
                    stream.write_u32(0).await.unwrap();
                } else {
                    let query_err = "error query";
                    stream.write_u32(query_err.len() as u32).await;
                    stream.write_all(query_err.as_bytes()).await;
                    stream.write_u32(0).await;
                }
            });
        }
    }
}
