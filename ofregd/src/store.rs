use anyhow::{Result, anyhow};
use ofreg_common::{OfregData, TABLE_NAME};
use rusqlite::Connection;
use std::sync::{LazyLock, Mutex};
use tracing::{error, info, warn};

pub const DB_FILE: &str = "/var/db/ofreg/ofreg.db";
const DB_IGNORE_PATH_PREFIX: [&str; 2] = ["/var/db/ofreg/", "/var/cache/fontconfig/"];

pub static OFREG_DB: LazyLock<Mutex<Connection>> = LazyLock::new(|| {
    let conn = db_open()
        .map_err(|_| error!("connection open error"))
        .unwrap();
    Mutex::new(conn)
});

fn is_ignore(path2: &str) -> bool {
    for ignore_path in DB_IGNORE_PATH_PREFIX {
        if let Some(idx) = path2.find(ignore_path)
            && idx == 0
        {
            return true;
        }
    }
    return false;
}

pub fn db_open() -> Result<Connection> {
    let conn_w = Connection::open(DB_FILE).map_err(|e| {
        warn!("{e}");
        anyhow!("")
    })?;
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
        "CREATE TABLE {} (cmd TEXT, op_file TEXT, time INTEGER)",
        TABLE_NAME
    );
    let table_count = conn_w
        .query_row(&query_exist, (), |row| row.get::<_, i32>(0))
        .map_err(|e| {
            warn!("{e}");
            anyhow!("")
        })?;
    if table_count == 0 {
        conn_w.execute(&create_table, ()).map_err(|e| {
            warn!("{e}");
            anyhow!("")
        })?;
    }
    info!("open db write connection");
    Ok(conn_w)
}

pub fn insert_item(conn: &Connection, item: &OfregData) -> Result<()> {
    if is_ignore(&item.op_file) {
        //print!("ignore: {}", item.op_file);
        return Ok(());
    }

    let insert = format!(
        "INSERT INTO {} (cmd, op_file, time) VALUES (?1, ?2, ?3)",
        TABLE_NAME
    );
    let _ = conn
        .execute(&insert, (&item.cmd, &item.op_file, item.time))
        .map_err(|e| warn!("item {:?} insert error: {e}", item));
    Ok(())
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
            time: 11,
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
