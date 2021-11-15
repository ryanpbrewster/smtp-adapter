use log::warn;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::protocol::{parse_command, Command};

pub mod protocol;

const AGENT: &str = "220 smtp.worse.email ESMTP Postfix\n";
pub async fn handle_connection(mut socket: TcpStream) {
    if let Err(err) = connection_handler(&mut socket).await {
        warn!("broken client: {}", err);
        let _ = socket
            .write_all(format!("error: {}\n", err).as_bytes())
            .await;
    }
}

enum SessionState {
    Initial,
    ReadingData { data: Vec<u8> },
}
async fn connection_handler(socket: &mut TcpStream) -> anyhow::Result<()> {
    let mut socket = BufReader::new(socket);
    socket.write_all(AGENT.as_bytes()).await?;
    let mut buf = Vec::with_capacity(1_024);

    let mut state = SessionState::Initial;
    loop {
        buf.clear();
        socket.read_until(b'\n', &mut buf).await?;
        if buf.is_empty() {
            return Ok(());
        }
        match state {
            SessionState::ReadingData { ref mut data } => {
                if buf == b".\n" || buf == b".\r\n" {
                    socket.write_all("250 Ok\n".as_bytes()).await?;
                    state = SessionState::Initial;
                } else {
                    data.extend(&buf);
                }
            }
            SessionState::Initial => {
                let cmd = parse_command(&buf)?;
                let reply = match cmd {
                    Command::Helo { domain } => {
                        format!("250 Hello {}, I am glad to meet you\n", domain)
                    }
                    Command::Ehlo { domain } => {
                        format!("250 Hello {}, I am glad to meet you\n", domain)
                    }
                    Command::MailFrom { .. } => "250 Ok\n".to_owned(),
                    Command::RcptTo { .. } => "250 Ok\n".to_owned(),
                    Command::Data => {
                        state = SessionState::ReadingData { data: Vec::new() };
                        "354 End data with <CR><LF>.<CR><LF>\n".to_owned()
                    }
                    Command::Quit => return Ok(()),
                };
                socket.write_all(reply.as_bytes()).await?;
            }
        };
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::AtomicU32;

    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::{TcpListener, TcpStream},
    };

    use crate::{handle_connection, AGENT};

    // This is a little janky, we consume a port per test.
    static TEST_PORT: AtomicU32 = AtomicU32::new(1984);
    async fn setup() -> anyhow::Result<BufReader<TcpStream>> {
        let port = TEST_PORT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let addr = format!("localhost:{}", port);
        let listener = TcpListener::bind(&addr).await?;
        let client = TcpStream::connect(&addr).await?;
        let (socket, _) = listener.accept().await?;
        tokio::spawn(handle_connection(socket));
        Ok(BufReader::new(client))
    }

    #[tokio::test]
    async fn smoke_test() -> anyhow::Result<()> {
        let mut client = setup().await?;
        let mut buf = String::with_capacity(1_024);

        client.read_line(&mut buf).await?;
        assert_eq!(buf, AGENT);
        buf.clear();

        client.write_all("HELO example.com\n".as_bytes()).await?;
        client.read_line(&mut buf).await?;
        assert_eq!(buf, "250 Hello example.com, I am glad to meet you\n");
        buf.clear();
        Ok(())
    }

    #[tokio::test]
    async fn quit_test() -> anyhow::Result<()> {
        let mut client = setup().await?;
        client.write_all("QUIT\n".as_bytes()).await?;

        let mut buf = String::new();
        // The server should close the stream, so we should get back an empty read eventually.
        while client.read_line(&mut buf).await? > 0 {}
        Ok(())
    }

    #[tokio::test]
    async fn data_test() -> anyhow::Result<()> {
        let mut client = setup().await?;
        let mut buf = String::new();

        client.read_line(&mut buf).await?;
        assert_eq!(buf, AGENT);
        buf.clear();

        client.write_all("DATA\n".as_bytes()).await?;
        client.read_line(&mut buf).await?;
        assert_eq!(buf, "354 End data with <CR><LF>.<CR><LF>\n");
        buf.clear();

        // TODO(rpb): this should be legal, we need to do state-tracking in the handler.
        client.write_all("Line 1\n".as_bytes()).await?;
        client.write_all("Line 2\n".as_bytes()).await?;
        client.write_all("Line 3\n".as_bytes()).await?;
        client.write_all(".\n".as_bytes()).await?;
        client.read_line(&mut buf).await?;
        assert_eq!(buf, "250 Ok\n");
        buf.clear();

        client.write_all("QUIT\n".as_bytes()).await?;
        while client.read_line(&mut buf).await? > 0 {}
        Ok(())
    }
}
