use smtp_adapter::handle_connection;

use tokio::net::TcpListener;

use log::info;

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

        tokio::spawn(handle_connection(socket));
    }
}
