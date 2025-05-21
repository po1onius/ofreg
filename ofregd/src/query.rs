use nix::unistd::Group;
use rusqlite::{ToSql, config::DbConfig};
use serde_json::json;
use std::{
    fs,
    os::unix::fs::{PermissionsExt, chown},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};
use tokio_rusqlite_new::Connection;

use crate::store::*;
use ofreg_common::SOCK_PATH;

pub struct QuerySrv {
    db_conn: Connection,
}

impl QuerySrv {
    pub async fn new_conn() -> Self {
        let db_conn = Connection::open(DB_FILE).await.unwrap();
        db_conn
            .call(|conn| {
                conn.execute_batch(
                    "PRAGMA journal_mode=WAL;
                PRAGMA synchronous=OFF;
                PRAGMA cache_size=-32000;
                PRAGMA busy_timeout=1000;",
                )
            })
            .await
            .unwrap();

        Self { db_conn }
    }

    pub async fn srv(&self) {
        fs::remove_file(SOCK_PATH).ok();
        let listener = UnixListener::bind(SOCK_PATH).unwrap();

        let user_group = Group::from_name("users").unwrap().unwrap();
        chown(SOCK_PATH, None, Some(user_group.gid.into())).unwrap();
        std::fs::set_permissions(SOCK_PATH, std::fs::Permissions::from_mode(0o660)).unwrap();

        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            println!("new connection!");
            let db_conn = self.db_conn.clone();
            tokio::spawn(async move {
                let payload_len = stream.read_u32().await.unwrap();
                let mut buf = vec![0u8; payload_len as usize];
                stream.read_exact(buf.as_mut_slice()).await.unwrap();
                println!(
                    "read {} bytes: {}",
                    payload_len,
                    str::from_utf8(buf.as_slice()).unwrap()
                );
                let result = db_conn
                    .call(move |conn| {
                        let mut stmt = conn.prepare(str::from_utf8(&buf).unwrap()).unwrap();
                        stmt.query_map([], |row| {
                            Ok(OfregData {
                                cmd: row.get(0).unwrap(),
                                op_file: row.get(1).unwrap(),
                                time: row.get(2).unwrap(),
                            })
                        })
                        .unwrap()
                        .collect::<Result<Vec<OfregData>, rusqlite::Error>>()
                    })
                    .await
                    .unwrap();
                for item in result {
                    let item_bin = json!(item).to_string();
                    stream.write_u32(item_bin.len() as u32).await.unwrap();
                    stream.write_all(item_bin.as_bytes()).await.unwrap();
                }
                stream.write_u32(0).await.unwrap();
            });
        }
    }
}
