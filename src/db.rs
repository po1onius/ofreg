use std::{
    num::NonZero,
    str::FromStr,
    sync::{LazyLock, Mutex, OnceLock, RwLock},
};

use rusqlite::Connection;

#[derive(Debug)]
pub struct OfregData {
    pub cmd: String,
    pub op_file: String,
    pub time: String,
}

const DB: &str = "/var/db/ofreg/ofreg.db";
const DB_PATH: &str = "/var/db/ofreg";
const DB_IGNORE: [&str; 2] = ["/var/db/ofreg/ofreg.db", "/var/db/ofreg/ofreg.db-journal"];
const TABLE_NAME: &str = "ofreg";

pub static OFREG_DB: LazyLock<Mutex<DbOp>> = LazyLock::new(|| Mutex::new(DbOp::open()));

pub struct DbOp {
    conn: Connection,
}

fn ignore_filter(path1: &str, path2: &str) -> bool {
    let path = std::path::Path::new(path1);
    if path.exists() {
        if path.is_dir() {
            let mut path_endwith_slash = String::from(path1);
            if path1.chars().last() != Some('/') {
                path_endwith_slash.push_str("/");
            }
            if let Some(idx) = path2.find(path_endwith_slash.as_str())
                && idx == 0
            {
                return true;
            }
        } else {
            if path1 == path2 {
                return true;
            }
        }
    }
    return false;
}

impl DbOp {
    pub fn open() -> Self {
        let db_path = std::path::Path::new(DB_PATH);
        if !db_path.exists() {
            std::fs::create_dir_all(db_path);
        }
        let conn = Connection::open(DB).unwrap();
        let query_exist = format!(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='{}'",
            TABLE_NAME
        );
        let create_table = format!(
            "CREATE TABLE {} (cmd TEXT, op_file TEXT, time TEXT)",
            TABLE_NAME
        );
        let table_count = conn
            .query_row(&query_exist, (), |row| row.get::<_, i32>(0))
            .unwrap();
        if table_count == 0 {
            conn.execute(&create_table, ()).unwrap();
        }
        Self { conn }
    }

    pub fn insert_item(&self, item: &OfregData) {
        for ignore_file in DB_IGNORE {
            if ignore_filter(ignore_file, &item.op_file) {
                return;
            }
        }
        let insert = format!(
            "INSERT INTO {} (cmd, op_file, time) VALUES (?1, ?2, ?3)",
            TABLE_NAME
        );
        self.conn
            .execute(&insert, (&item.cmd, &item.op_file, &item.time))
            .unwrap();
    }
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
