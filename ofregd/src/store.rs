use ofreg_common::TABLE_NAME;
use rusqlite::Connection;
use serde::Serialize;
use std::{
    num::NonZero,
    str::FromStr,
    sync::{LazyLock, Mutex, OnceLock, RwLock},
};
use tracing::{error, info, warn};

#[derive(Debug, Serialize)]
pub struct OfregData {
    pub cmd: String,
    pub op_file: String,
    pub time: String,
}

pub const DB_FILE: &str = "/var/db/ofreg/ofreg.db";
pub const DB_PATH: &str = "/var/db/ofreg";
const DB_IGNORE: [&str; 1] = ["/var/db/ofreg"];

pub static OFREG_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let conn = db_open();
    Mutex::new(conn)
});

fn is_ignore(path2: &str) -> bool {
    for ignore_path in DB_IGNORE {
        let path = std::path::Path::new(ignore_path);
        if path.exists() {
            if path.is_dir() {
                let mut path_endwith_slash = String::from(ignore_path);
                if ignore_path.chars().last() != Some('/') {
                    path_endwith_slash.push_str("/");
                }
                if let Some(idx) = path2.find(path_endwith_slash.as_str())
                    && idx == 0
                {
                    return true;
                }
            } else {
                if ignore_path == path2 {
                    return true;
                }
            }
        }
    }
    return false;
}

pub fn db_open() -> Connection {
    let db_path = std::path::Path::new(DB_PATH);
    if !db_path.exists() {
        std::fs::create_dir_all(db_path)
            .map_err(|e| error!("{e}"))
            .unwrap();
        info!("create db path");
    }
    let conn_w = Connection::open(DB_FILE)
        .map_err(|e| error!("{e}"))
        .unwrap();
    let _ = conn_w
        .execute_batch(
            "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA cache_size=-100000;
         PRAGMA busy_timeout=5000;
         PRAGMA foreign_keys=OFF;",
        )
        .map_err(|e| warn!("{e}"));

    let query_exist = format!(
        "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
        TABLE_NAME
    );
    let create_table = format!(
        "CREATE TABLE {} (cmd TEXT, op_file TEXT, time TEXT)",
        TABLE_NAME
    );
    let table_count = conn_w
        .query_row(&query_exist, (), |row| row.get::<_, i32>(0))
        .map_err(|e| error!("{e}"))
        .unwrap();
    if table_count == 0 {
        conn_w
            .execute(&create_table, ())
            .map_err(|e| error!("{e}"))
            .unwrap();
    }
    info!("open db write connection");
    conn_w
}

pub fn insert_item(conn: &Connection, item: &OfregData) {
    if is_ignore(&item.op_file) {
        return;
    }
    let insert = format!(
        "INSERT INTO {} (cmd, op_file, time) VALUES (?1, ?2, ?3)",
        TABLE_NAME
    );
    let _ = conn
        .execute(&insert, (&item.cmd, &item.op_file, &item.time))
        .map_err(|e| warn!("item {:?} insert error: {e}", item));
}

#[cfg(test)]
mod test {
    use super::*;
    use rusqlite::Connection;
    #[test]
    fn sql_test() {
        let conn = Connection::open("test.db").unwrap();
        conn.execute(
            format!("CREATE TABLE ofreg_data (cmd  TEXT, op_file TEXT, time TEXT)").as_str(),
            (),
        )
        .unwrap();
        let ofreg_data = OfregData {
            cmd: "qq".into(),
            op_file: "/etc/sudoers".into(),
            time: "2025-05-05 18:35:00".into(),
        };

        conn.execute(
            "INSERT INTO ofreg_data(cmd, op_file, time) VALUES(?1, ?2, ?3)",
            (ofreg_data.cmd, ofreg_data.op_file, ofreg_data.time),
        )
        .unwrap();

        let mut stmt = conn
            .prepare("SELECT cmd, op_file, time FROM ofreg_data")
            .unwrap();
        let data_iter = stmt
            .query_map([], |row| {
                Ok(OfregData {
                    cmd: row.get(0).unwrap(),
                    op_file: row.get(1).unwrap(),
                    time: row.get(2).unwrap(),
                })
            })
            .unwrap();

        for line in data_iter {
            println!("Found ofreg data {:?}", line.unwrap());
        }
    }

    #[test]
    fn strop_test() {
        let s = "/home/aaa/bbb";
        assert_eq!(s.find("/home/aaa").unwrap(), 0);
    }
}
