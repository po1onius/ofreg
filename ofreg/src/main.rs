use clap::Parser;
use ofreg_common::{OfregData, SOCK_PATH, TABLE_NAME};
use serde_json::Value;
use tabled::{Table, builder::Builder};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// query command
    #[arg(short, long, conflicts_with = "file")]
    cmd: Option<String>,

    /// query file
    #[arg(short, long, conflicts_with = "cmd")]
    file: Option<String>,

    /// query number
    #[arg(short, default_value_t = 10)]
    n: u32,
}

const KEYS: [&str; 3] = ["cmd", "op_file", "time"];

fn table_fmt(json_str: &str, select: Option<&str>) {
    let data: Vec<Value> = serde_json::from_str(json_str).unwrap();
    let mut builder = Builder::default();

    let t = if let Some(s) = select { s } else { "" };
    let table_header = KEYS
        .iter()
        .filter(|&&s| s != t)
        .map(|s| s.to_string())
        .collect::<Vec<_>>();

    builder.push_record(table_header);

    for obj in data {
        if let Value::Object(map) = obj {
            let mut vs = Vec::new();
            for (_, val) in map {
                let value_str = match val {
                    Value::String(s) => s.clone(),
                    _ => val.to_string(),
                };
                vs.push(value_str);
            }
            builder.push_record(vs);
        }
    }

    println!("{}", builder.build());
}

fn select_stmt(target: Option<&str>) -> String {
    let t = if let Some(s) = target { s } else { "" };
    let show_filed = KEYS
        .iter()
        .map(|&s| s.to_string())
        .filter(|s| s != t)
        .map(|s| format!("'{}',{}", s, s))
        .collect::<Vec<_>>()
        .join(",");

    let select = format!(
        "select json_group_array(json_object({})) FROM {} LIMIT 10",
        show_filed, TABLE_NAME
    );
    return select;
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut select = select_stmt(None);
    let mut select_target = None;
    if let Some(cmd) = args.cmd {
        select = select_stmt(Some(cmd.as_str()));
        select_target = Some("cmd");
    }

    if let Some(file) = args.file {
        select = select_stmt(Some(file.as_str()));
        select_target = Some("op_file");
    }

    println!("{select}");

    let mut stream = UnixStream::connect(SOCK_PATH).await.unwrap();
    println!("{select}");
    stream.write_u32(select.len() as u32).await.unwrap();
    stream.write_all(select.as_bytes()).await.unwrap();

    loop {
        let item_len = stream.read_u32().await.unwrap();
        if item_len == 0 {
            break;
        }
        let mut buf = vec![0; item_len as usize];
        stream.read_exact(buf.as_mut_slice()).await.unwrap();
        table_fmt(str::from_utf8(buf.as_slice()).unwrap(), select_target);
    }
}
