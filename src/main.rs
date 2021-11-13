use std::net::{IpAddr, SocketAddr};
use std::sync::{Arc, Mutex};
use anyhow::anyhow;

use mailin::{Action, Handler};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

use log::{info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();
    let address = "127.0.0.1:8080";
    let listener = TcpListener::bind(address).await?;
    println!("Listening on: {}", address);

    loop {
        // Asynchronously wait for an inbound socket.
        let (socket, addr) = listener.accept().await?;
        info!("client connected from {}", addr);

        tokio::spawn(async move {
            match handle_connection(socket, addr.ip()).await {
                Ok(_) => info!("client disconnected cleanly from {}", addr),
                Err(err) => warn!("broken client {}: {}", addr, err),
            }
        });
    }
}

async fn handle_connection(mut socket: TcpStream, addr: IpAddr) -> anyhow::Result<()> {
    let mut buf = vec![0; 1024];
    // TODO: why isn't `Session: Send`?
    let session = {
        let s = mailin::SessionBuilder::new("worse-email").build(addr, MyHandler);
        Arc::new(Mutex::new(s))
    };

    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        let response = session.lock().unwrap().process(&buf);
        match response.action {
            Action::Close => return Ok(()),
            Action::NoReply => continue,
            Action::UpgradeTls => return Err(anyhow!("tls unsupported")),
            Action::Reply => socket.write_all(&buf[0..n]).await?,
        };
    }
}

struct MyHandler;
impl Handler for MyHandler {

}