mod time;

use clap::Parser;
use ofreg_common::{OfregData, Query, SOCK_PATH};

use tabled::Table;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[derive(Debug, clap::Parser)]
pub struct QueryArg {
    /// 按命令查找
    #[arg(short)]
    pub cmd: Option<String>,
    /// 按文件查找
    #[arg(short)]
    pub file: Option<String>,
    /// 按时间段查找（开始时间）
    #[arg(short = 'b')]
    pub time_begin: Option<String>,
    /// 按时间段查找（结束时间）
    #[arg(short = 'e')]
    pub time_end: Option<String>,
    /// 列出记录数量
    #[arg(short, default_value_t = 10)]
    pub num: u32,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, tabled::Tabled)]
pub struct OfregDataTab {
    pub cmd: String,
    pub op_file: String,
    pub time: String,
}

#[tokio::main]
async fn main() {
    let args = QueryArg::parse();
    let query = Query {
        cmd: args.cmd,
        file: args.file,
        time_begin: args
            .time_begin
            .map(|s| time::human_time_to_sec(s.as_str()).expect("time fmt error")),
        time_end: args
            .time_end
            .map(|s| time::human_time_to_sec(s.as_str()).expect("time fmt error")),
        num: args.num,
    };

    let select = serde_json::to_string(&query).expect("arg error");

    // println!("{select}");

    let mut stream = UnixStream::connect(SOCK_PATH).await.unwrap();
    stream.write_u32(select.len() as u32).await.unwrap();
    stream.write_all(select.as_bytes()).await.unwrap();

    let item_len = stream.read_u32().await.unwrap();

    let mut buf = vec![0; item_len as usize];
    stream.read_exact(buf.as_mut_slice()).await.unwrap();

    let data: Vec<OfregData> =
        serde_json::from_slice(buf.as_slice()).expect("query result fmt error");

    let tab = data
        .into_iter()
        .map(|o| OfregDataTab {
            cmd: o.cmd,
            op_file: o.op_file,
            time: time::sec_to_human_time(o.time).expect("time conver error"),
        })
        .collect::<Vec<_>>();
    println!("{}", Table::new(tab));
}
