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

fn print_json_array(json_str: &str) {
    let data: Vec<Value> = serde_json::from_str(json_str).unwrap();
    let mut builder = Builder::default();

    // 添加表头
    builder.push_record(vec!["Key", "Value"]);

    // 动态解析每个对象
    for obj in data {
        if let Value::Object(map) = obj {
            for (key, val) in map {
                let value_str = match val {
                    Value::String(s) => s.clone(),
                    _ => val.to_string(),
                };
                builder.push_record(vec![key, value_str]);
            }
        }
    }

    println!("{}", builder.build());
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let mut select = format!("SELECT * FROM {} LIMIT 10", TABLE_NAME);
    if let Some(cmd) = args.cmd {
        select = format!(
            "SELECT op_file, time FROM {} WHERE cmd = '{}' LIMIT 10",
            TABLE_NAME, cmd
        );
    }

    if let Some(file) = args.file {
        select = format!(
            "SELECT cmd, time FROM {} WHERE op_file = '{}' LIMIT 10",
            TABLE_NAME, file
        );
    }

    let select = "select json_group_array(json_object('cmd', cmd, 'file', op_file)) from ofreg";

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
        print_json_array(str::from_utf8(buf.as_slice()).unwrap());
    }
}
