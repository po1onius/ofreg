use ofreg_common::{SOCK_PATH, TABLE_NAME};
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

    loop {
        let item_len = stream.read_u32().await.unwrap();
        if item_len == 0 {
            return;
        }
        let mut buf = vec![0; item_len as usize];
        stream.read_exact(buf.as_mut_slice()).await.unwrap();
        println!("{}", str::from_utf8(buf.as_slice()).unwrap());
    }
}
