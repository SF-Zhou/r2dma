use derse::Serialization;
use r2dma::*;
use std::env;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9999".to_string());

    let config = Config::default();
    let manager = Manager::init(&config).unwrap();
    let send_socket = manager.create_socket().unwrap();
    println!("recv socket: {:#?}", send_socket);
    let local_endpoint = send_socket.endpoint();
    println!("endpoint: {:#?}", local_endpoint);
    let bytes = local_endpoint.serialize::<derse::DownwardBytes>().unwrap();
    println!("{:#?}", bytes.len());
    let len = bytes.len();

    let mut socket = TcpStream::connect(addr).await?;
    socket.write_all(bytes.as_ref()).await?;

    let mut buf = vec![0; len];
    let _ = socket
        .read(&mut buf)
        .await
        .expect("failed to read data from socket");

    let remote_endpoint = Endpoint::deserialize(buf.as_ref()).unwrap();
    println!("remote endpoint: {:#?}", remote_endpoint);
    send_socket.ready(&remote_endpoint).unwrap();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    let mut send_memory = manager.allocate_buffer().unwrap();
    println!("send memory: {:#?}", send_memory);
    send_memory.as_mut().fill(0x23);

    // let result = send_socket.send(send_memory).unwrap().await;
    // println!("{:#?}", result);

    Ok(())
}
