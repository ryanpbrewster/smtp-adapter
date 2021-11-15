use log::warn;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::protocol::{parse_command, Command};

pub mod protocol;

const AGENT: &str = "220 smtp.worse.email ESMTP Postfix\n";
pub async fn handle_connection(mut socket: TcpStream) {
    if let Err(err) = connection_handler(&mut socket).await {
        warn!("broken client: {}", err);
        let _ = socket.write_all(format!("error: {}", err).as_bytes()).await;
    }
}
async fn connection_handler(socket: &mut TcpStream) -> anyhow::Result<()> {
    socket.write_all(AGENT.as_bytes()).await?;
    let mut buf = vec![0; 1024];

    // In a loop, read data from the socket and write the data back.
    loop {
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            return Ok(());
        }
        let cmd = parse_command(&buf[..n])?;
        let reply = match cmd {
            Command::Helo { name } => format!("250 Hello {}, I am glad to meet you\n", name),
            Command::Quit => return Ok(()),
        };
        socket.write_all(reply.as_bytes()).await?;
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicU32;

    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use crate::{handle_connection, AGENT};

    // This is a little janky, we consume a port per test.
    static TEST_PORT: AtomicU32 = AtomicU32::new(1984);
    async fn setup() -> anyhow::Result<TcpStream> {
        let port = TEST_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let addr = format!("localhost:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        let client = TcpStream::connect(&addr).await?;
        let (socket, _) = listener.accept().await?;
        tokio::spawn(handle_connection(socket));
        Ok(client)
    }

    #[tokio::test]
    async fn smoke_test() -> anyhow::Result<()> {
        let mut client = setup().await?;
        let mut buf: Vec<u8> = vec![0; 1_024];

        let n = client.read(&mut buf).await?;
        assert_eq!(std::str::from_utf8(&buf[..n])?, AGENT);

        client.write_all("HELO example.com".as_bytes()).await?;
        let n = client.read(&mut buf).await?;
        assert_eq!(
            std::str::from_utf8(&buf[..n])?,
            "250 Hello example.com, I am glad to meet you\n"
        );
        Ok(())
    }

    #[tokio::test]
    async fn quit_test() -> anyhow::Result<()> {
        let mut client = setup().await?;
        client.write_all("QUIT".as_bytes()).await?;

        let mut buf: Vec<u8> = vec![0; 1_024];
        // The server should close the stream, so we should get back an empty read eventually.
        while client.read(&mut buf).await? > 0 {}
        Ok(())
    }
}
