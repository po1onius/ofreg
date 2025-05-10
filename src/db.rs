#[derive(Debug)]
struct OfregData {
    cmd: String,
    op_file: String,
}

#[cfg(test)]
mod test {
    use super::*;
    use rusqlite::Connection;
    #[test]
    fn sql_test() {
        let conn = Connection::open("test.db").unwrap();
        conn.execute(
            "CREATE TABLE ofreg_data (
            cmd   VERVHAR(255),
            op_file VERCHAR(255)
        )",
            (),
        )
        .unwrap();
        let ofreg_data = OfregData {
            cmd: "qq".into(),
            op_file: "/etc/sudoers".into(),
        };

        conn.execute(
            "INSERT INTO ofreg_data (cmd, op_file) VALUES (?1, ?2)",
            (ofreg_data.cmd, ofreg_data.op_file),
        )
        .unwrap();

        let mut stmt = conn.prepare("SELECT cmd, op_file FROM ofreg_data").unwrap();
        let data_iter = stmt
            .query_map([], |row| {
                Ok(OfregData {
                    cmd: row.get(0).unwrap(),
                    op_file: row.get(1).unwrap(),
                })
            })
            .unwrap();

        for line in data_iter {
            println!("Found ofreg data {:?}", line.unwrap());
        }
    }
}
