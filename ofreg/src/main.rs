use ofreg_common::{OfregData, SOCK_PATH, TABLE_NAME};
use tabled::Table;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

#[tokio::main]
async fn main() {
    let mut stream = UnixStream::connect(SOCK_PATH).await.unwrap();

    let select = format!("SELECT * FROM {} LIMIT 10", TABLE_NAME);

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
        let data = serde_json::from_slice::<OfregData>(buf.as_slice()).unwrap();
        ofreg_data_vec.push(data);
    }

    let table = Table::new(ofreg_data_vec);
    println!("{table}");
}
