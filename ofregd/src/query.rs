use anyhow::{Error, Result, anyhow};
use nix::unistd::Group;
use rusqlite::{ToSql, config::DbConfig};
use serde_json::json;
use std::{
    fs, io,
    os::unix::fs::{PermissionsExt, chown},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
use tokio_rusqlite_new::Connection;
use tracing::{error, info, warn};

use crate::store::*;
use ofreg_common::{OfregData, SOCK_PATH};

pub struct QuerySrv {
    db_conn: Connection,
}

impl QuerySrv {
    pub async fn new_conn() -> Result<Self> {
        let db_conn = Connection::open(DB_FILE).await.map_err(|e| {
            warn!("db open connection error: {}", e);
            anyhow!("")
        })?;

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

        Ok(Self { db_conn })
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

                if let Ok(s) = str::from_utf8(buf.as_slice()) {
                    info!("read {} bytes: {}", payload_len, s);
                } else {
                    warn!("cli send bad query cmd");
                    return;
                }

                match db_conn
                    .call(move |conn| -> Result<String, rusqlite::Error> {
                        let query_str = str::from_utf8(buf.as_slice()).unwrap();
                        let s: String = conn.query_row(query_str, [], |row| row.get(0))?;
                        Ok(s)
                    })
                    .await
                {
                    Ok(result) => {
                        write_frame(&mut stream, result.as_bytes()).await;
                        stream.write_u32(0).await.piperr();
                        info!("response to cli query result");
                    }
                    Err(e) => {
                        warn!("{e}");
                        stream.write_u32(0).await.piperr();
                    }
                }
            });
        }
    }
}

trait IoErrHandle {
    fn piperr(&self) {
        panic!("custom unwrap");
    }
}

impl<T> IoErrHandle for Result<T, io::Error> {
    fn piperr(&self) {
        if let Err(e) = self {
            if e.kind() != io::ErrorKind::BrokenPipe {
                error!("{e}");
                panic!();
            }
            warn!("{e}");
        }
    }
}

async fn write_frame(stream: &mut UnixStream, data: &[u8]) {
    stream.write_u32(data.len() as u32).await.piperr();
    stream.write_all(data).await.piperr();
}
