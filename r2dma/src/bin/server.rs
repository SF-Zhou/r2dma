use derse::Serialization;
use r2dma::{Buffer, Cards, Endpoint, SendRecv, Socket};
use std::env;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9999".to_string());

    let listener = TcpListener::bind(&addr).await?;
    println!("Listening on: {}", addr);

    let cards = Cards::open().unwrap();
    let event_loop = cards.event_loops.first().unwrap();
    println!("{:#?}", Socket::create(event_loop).unwrap());
    let recv_socket = Socket::create(event_loop).unwrap();
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

    let mut recv_memory = Buffer::new(&cards.cards, 1048576).unwrap();
    println!("recv memory: {:#?}", recv_memory);
    recv_memory.as_mut().fill(0);

    let recv = SendRecv {
        is_recv: true,
        socket: recv_socket.clone(),
        mem: &recv_memory,
        waker: None,
        result: None,
    };

    let result = recv.await;
    println!("result is {:#?}", result);

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    Ok(())
}
