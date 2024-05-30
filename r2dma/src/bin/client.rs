use derse::Serialization;
use r2dma::{Buffer, Cards, Endpoint, SendRecv, Socket};
use std::env;
use std::error::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let addr = env::args()
        .nth(1)
        .unwrap_or_else(|| "0.0.0.0:9999".to_string());

    let cards = Cards::open().unwrap();
    let event_loop = cards.event_loops.first().unwrap();
    println!("{:#?}", Socket::create(event_loop).unwrap());
    let send_socket = Socket::create(event_loop).unwrap();
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

    let mut send_memory = Buffer::new(&cards.cards, 1048576).unwrap();
    println!("send memory: {:#?}", send_memory);
    send_memory.as_mut().fill(0x23);

    let send = SendRecv {
        is_recv: false,
        socket: send_socket.clone(),
        mem: &send_memory,
        waker: None,
        result: None,
    };
    let result = send.await;
    println!("{:#?}", result);

    Ok(())
}
