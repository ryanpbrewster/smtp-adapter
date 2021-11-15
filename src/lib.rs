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
        };
        println!("writing: {}", reply);
        socket.write_all(reply.as_bytes()).await?;
    }
}

#[cfg(test)]
mod test {
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::{TcpListener, TcpStream},
    };

    use crate::{handle_connection, AGENT};

    const ADDR: &str = "localhost:1984";

    #[tokio::test]
    async fn smoke_test() -> anyhow::Result<()> {
        let listener = TcpListener::bind(ADDR).await?;
        let mut client = TcpStream::connect(ADDR).await?;
        let (socket, _) = listener.accept().await?;
        let job = tokio::spawn(handle_connection(socket));
        let mut buf: Vec<u8> = vec![0; 1_024];

        let n = client.read(&mut buf).await?;
        assert_eq!(std::str::from_utf8(&buf[..n])?, AGENT);

        client.write_all("HELO example.com".as_bytes()).await?;
        let n = client.read(&mut buf).await?;
        assert_eq!(
            std::str::from_utf8(&buf[..n])?,
            "250 Hello example.com, I am glad to meet you"
        );

        std::mem::drop(client);
        job.await?;
        Ok(())
    }
}
