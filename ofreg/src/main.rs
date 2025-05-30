use clap::Parser;
use ofreg_common::{OfregData, SOCK_PATH, TABLE_NAME};
use tabled::Table;
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

    /// query line number
    #[arg(short, default_value_t = 10)]
    n: u32,
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

    let mut stream = UnixStream::connect(SOCK_PATH).await.unwrap();
    println!("{select}");
    stream.write_u32(select.len() as u32).await.unwrap();
    stream.write_all(select.as_bytes()).await.unwrap();

    let mut ofreg_data_vec = Vec::new();

    loop {
        let item_len = stream.read_u32().await.unwrap();
        if item_len == 0 {
            break;
        }
        let mut buf = vec![0; item_len as usize];
        stream.read_exact(buf.as_mut_slice()).await.unwrap();
        if let Ok(data) = serde_json::from_slice::<OfregData>(buf.as_slice()) {
            ofreg_data_vec.push(data);
        } else {
            println!("{}", str::from_utf8(buf.as_slice()).unwrap());
        }
    }
    if ofreg_data_vec.len() == 0 {
        return;
    }
    let table = Table::new(ofreg_data_vec);
    println!("{table}");
}
