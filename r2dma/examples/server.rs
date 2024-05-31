use derse::Serialization;
use r2dma::*;
use std::env;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> std::result::Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9999".to_string());

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    let config = Config::default();
    let manager = Manager::init(&config).unwrap();
    let recv_socket = manager.create_socket().unwrap();
    println!("recv socket: {:#?}", recv_socket);
    let local_endpoint = recv_socket.endpoint();
    println!("endpoint: {:#?}", local_endpoint);
    let bytes = local_endpoint.serialize::<derse::DownwardBytes>().unwrap();
    println!("{:#?}", bytes.len());
    let len = bytes.len();

    let (mut socket, _) = listener.accept().await?;

    let mut buf = vec![0; len];
    let _ = socket
        .read(&mut buf)
        .await
        .expect("failed to read data from socket");

    let remote_endpoint = Endpoint::deserialize(buf.as_ref()).unwrap();
    println!("remote endpoint: {:#?}", remote_endpoint);
    recv_socket.ready(&remote_endpoint).unwrap();

    socket
        .write_all(&bytes[..])
        .await
        .expect("failed to write data to socket");

    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    let mut recv_memory = manager.allocate_buffer().unwrap();
    println!("recv memory: {:#?}", recv_memory);
    recv_memory.as_mut().fill(0);

    let result = recv_socket.recv(recv_memory).unwrap().await;
    println!("result is {:#?}", result);

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}
