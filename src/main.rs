use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use log::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let addr = "127.0.0.1:8080";
    let listener = TcpListener::bind(addr).await?;
    println!("Listening on: {}", addr);

    loop {
        // Asynchronously wait for an inbound socket.
        let (socket, a) = listener.accept().await?;
        info!("client connected on {}", a);

        tokio::spawn(async move {
            match handle_connection(socket).await {
                Ok(_) => info!("client disconnected cleanly from {}", a),
                Err(e) => warn!("broken client {}: {}", a, e),
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream) -> anyhow::Result<()> {
    let mut buf = vec![0; 1024];

    // In a loop, read data from the socket and write the data back.
    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        socket.write_all(&buf[0..n]).await?;
    }
}
