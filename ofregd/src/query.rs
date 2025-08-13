use anyhow::{Result, anyhow};
use nix::unistd::Group;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, chown},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{UnixListener, UnixStream},
};
use tokio_rusqlite_new::Connection;
use tracing::{error, info, warn};

use crate::store::*;
use ofreg_common::{OfregData, Query, SOCK_PATH, TABLE_NAME};

pub struct QuerySrv {
    db_conn: Connection,
}

impl QuerySrv {
    pub async fn new_conn() -> Result<Self> {
        let db_conn = Connection::open(DB_FILE).await?;

        let _ = db_conn
            .call(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode=WAL;
                PRAGMA synchronous=OFF;
                PRAGMA cache_size=-32000;
                PRAGMA busy_timeout=1000;",
                )
            })
            .await?;

        info!("db connect and init");

        Ok(Self { db_conn })
    }

    pub async fn srv(&self) -> Result<()> {
        fs::remove_file(SOCK_PATH).ok();
        let listener =
            UnixListener::bind(SOCK_PATH).map_err(|_| anyhow!("unix socket path bind error"))?;

        if let Ok(Some(user_group)) = Group::from_name("users") {
            let _ = chown(SOCK_PATH, None, Some(user_group.gid.into()))
                .map_err(|e| warn!("{}", e.to_string()));
            let _ = std::fs::set_permissions(SOCK_PATH, std::fs::Permissions::from_mode(0o660))
                .map_err(|e| warn!("{}", e.to_string()));
        } else {
            warn!("can't get \"users\" group info, it may cause ofreg cli can't work");
        }

        loop {
            let Ok((mut stream, _)) = listener
                .accept()
                .await
                .map_err(|e| error!("{}", e.to_string()))
            else {
                continue;
            };

            let db_conn = self.db_conn.clone();
            tokio::spawn(async move {
                let Ok(payload_len) = stream
                    .read_u32()
                    .await
                    .map_err(|e| error!("{}", e.to_string()))
                else {
                    return;
                };
                let mut buf = vec![0u8; payload_len as usize];
                if stream
                    .read_exact(buf.as_mut_slice())
                    .await
                    .map_err(|e| error!("{}", e.to_string()))
                    .is_err()
                {
                    return;
                }

                let Ok(query_item) = String::from_utf8(buf) else {
                    warn!("cli send bad query cmd");
                    return;
                };

                let Ok(query) = serde_json::from_str::<Query>(query_item.as_str()) else {
                    warn!("cli send bad query cmd");
                    return;
                };

                let query_str = format!("select * from {} {}", TABLE_NAME, sqlcat(&query));
                // println!("{query_str}");

                match db_conn
                    .call(move |conn| {
                        let mut stmt = conn.prepare(query_str.as_str())?;
                        let data = stmt
                            .query_map([], |row| {
                                Ok(OfregData {
                                    cmd: row.get(0)?,
                                    op_file: row.get(1)?,
                                    time: row.get(2)?,
                                })
                            })?
                            .collect::<Result<Vec<OfregData>, rusqlite::Error>>()?;
                        Ok::<_, rusqlite::Error>(data)
                    })
                    .await
                {
                    Ok(result) => {
                        if let Ok(data) = serde_json::to_string(&result) {
                            if write_frame(&mut stream, data.as_bytes())
                                .await
                                .map_err(|e| error!("success query but response error => {}", e))
                                .is_err()
                            {
                                return;
                            };
                            info!("response to cli query result");
                        } else {
                            warn!("query result serde err");
                        }
                    }
                    Err(e) => {
                        warn!("{e}");
                    }
                }
                stream
                    .write_u32(0)
                    .await
                    .map_err(|e| error!("write finish flag error => {}", e))
                    .err();
            });
        }
    }
}

async fn write_frame(stream: &mut UnixStream, data: &[u8]) -> anyhow::Result<()> {
    stream.write_u32(data.len() as u32).await?;
    stream.write_all(data).await?;
    Ok(())
}

fn sqlcat(query: &Query) -> String {
    let mut conds = vec![];
    if let Some(cmd) = &query.cmd {
        conds.push(format!("cmd = '{}'", cmd));
    }
    if let Some(file) = &query.file {
        conds.push(format!("op_file like '{}%'", file));
    }
    if let Some(tb) = &query.time_begin {
        conds.push(format!("time > {}", tb));
    }
    if let Some(te) = &query.time_end {
        conds.push(format!("time < {}", te));
    }
    let mut conds_str = String::new();
    if !conds.is_empty() {
        conds_str.push_str("where ");
        conds_str.push_str(conds.join(" and ").to_string().as_str());
    }
    conds_str.push_str(format!(" order by time desc limit {}", query.num).as_str());
    conds_str
}
