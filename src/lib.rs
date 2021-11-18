use anyhow::anyhow;
use log::warn;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

use crate::protocol::{parse_command, Command};

pub mod protocol;

const AGENT: &str = "220 smtp.worse.email ESMTP Postfix\n";
pub async fn handle_connection(mut socket: TcpStream) {
    if let Err(err) = connection_handler(&mut socket).await {
        warn!("broken client: {}", err);
        let _ = socket.write_all(format!("{}\n", err).as_bytes()).await;
    }
}

enum SessionState {
    Initial,
    Greeted,
    ReadingData { data: Vec<u8> },
}
enum Response {
    Quit,
    Continue,
    Reply(String),
}
impl SessionState {
    fn handle(&mut self, line: &[u8]) -> anyhow::Result<Response> {
        let response = match self {
            SessionState::Initial => {
                let cmd = parse_command(line)?;
                match cmd {
                    Command::Quit => Response::Quit,
                    Command::Helo { domain } | Command::Ehlo { domain } => {
                        *self = SessionState::Greeted;
                        Response::Reply(format!("250 Hello {}, I am glad to meet you\n", domain))
                    }
                    Command::MailFrom { .. } | Command::RcptTo { .. } | Command::Data => {
                        return Err(anyhow!("500 You haven't said hello yet!\n"));
                    }
                }
            }
            SessionState::Greeted => {
                let cmd = parse_command(line)?;
                match cmd {
                    Command::Quit => Response::Quit,
                    Command::Helo { .. } | Command::Ehlo { .. } => {
                        return Err(anyhow!("500 You already said hello...\n"));
                    }
                    Command::MailFrom { .. } | Command::RcptTo { .. } => {
                        Response::Reply("250 Ok\n".to_owned())
                    }
                    Command::Data => {
                        *self = SessionState::ReadingData { data: Vec::new() };
                        Response::Reply("354 End data with <CR><LF>.<CR><LF>\n".to_owned())
                    }
                }
            }
            SessionState::ReadingData { data } => {
                if line == b".\n" || line == b".\r\n" {
                    *self = SessionState::Initial;
                    Response::Reply("250 Ok\n".to_owned())
                } else {
                    data.extend(line);
                    Response::Continue
                }
            }
        };
        Ok(response)
    }
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
        match state.handle(&buf)? {
            Response::Quit => return Ok(()),
            Response::Continue => continue,
            Response::Reply(reply) => socket.write_all(reply.as_bytes()).await?,
        };
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::{AtomicU32, Ordering};

    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::{TcpListener, TcpStream},
    };

    use crate::{handle_connection, AGENT};
    use async_trait::async_trait;

    #[async_trait]
    trait SimpleTestStream {
        async fn line(&mut self) -> anyhow::Result<String>;
    }
    #[async_trait]
    impl SimpleTestStream for BufReader<TcpStream> {
        async fn line(&mut self) -> anyhow::Result<String> {
            let mut buf = String::new();
            self.read_line(&mut buf).await?;
            Ok(buf)
        }
    }

    // This is a little janky, we consume a port per test.
    static TEST_PORT: AtomicU32 = AtomicU32::new(1984);
    async fn setup() -> anyhow::Result<BufReader<TcpStream>> {
        let port = TEST_PORT.fetch_add(1, Ordering::SeqCst);
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

        assert_eq!(client.line().await?, AGENT);

        client.write_all("HELO yo.dog\n".as_bytes()).await?;
        assert_eq!(
            client.line().await?,
            "250 Hello yo.dog, I am glad to meet you\n"
        );

        Ok(())
    }

    #[tokio::test]
    async fn quit_test() -> anyhow::Result<()> {
        let mut client = setup().await?;
        client.write_all("QUIT\n".as_bytes()).await?;

        // The server should close the stream, so we should get back an empty read eventually.
        while !client.line().await?.is_empty() {}
        Ok(())
    }

    #[tokio::test]
    async fn data_test() -> anyhow::Result<()> {
        let mut client = setup().await?;
        assert_eq!(client.line().await?, AGENT);

        client.write_all("HELO yo.dog\n".as_bytes()).await?;
        assert_eq!(
            client.line().await?,
            "250 Hello yo.dog, I am glad to meet you\n"
        );

        client.write_all("DATA\n".as_bytes()).await?;
        assert_eq!(
            client.line().await?,
            "354 End data with <CR><LF>.<CR><LF>\n"
        );

        client.write_all("Line 1\n".as_bytes()).await?;
        client.write_all("Line 2\n".as_bytes()).await?;
        client.write_all("Line 3\n".as_bytes()).await?;
        client.write_all(".\n".as_bytes()).await?;
        assert_eq!(client.line().await?, "250 Ok\n");

        client.write_all("QUIT\n".as_bytes()).await?;
        while !client.line().await?.is_empty() {}

        Ok(())
    }

    #[tokio::test]
    async fn data_cannot_be_invoked_without_helo() -> anyhow::Result<()> {
        let mut client = setup().await?;

        assert_eq!(client.line().await?, AGENT);

        client.write_all("DATA\n".as_bytes()).await?;
        assert_eq!(client.line().await?, "500 You haven't said hello yet!\n");
        Ok(())
    }
}
