use rusqlite::{ToSql, config::DbConfig};
use serde_json::json;
use std::fs;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixListener,
};
use tokio_rusqlite_new::Connection;

use crate::store::*;

pub struct QuerySrv {
    db_conn: Connection,
}

impl QuerySrv {
    async fn new_conn() -> Self {
        let db_conn = Connection::open(DB).await.unwrap();
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
        let socket_path = "/run/user/1000/ofreg.sock";
        fs::remove_file(socket_path).ok();
        let listener = UnixListener::bind(socket_path).unwrap();

        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            let db_conn = self.db_conn.clone();
            tokio::spawn(async move {
                let payload_len = stream.read_u32().await.unwrap();
                let mut buf = vec![0u8; payload_len as usize];
                stream.read_exact(buf.as_mut_slice()).await.unwrap();
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
