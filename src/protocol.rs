use anyhow::anyhow;
use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while1},
    character::complete::multispace0,
    combinator::{all_consuming, value},
    sequence::{delimited, terminated},
    AsChar, IResult,
};

#[derive(PartialEq, Eq, Debug)]
pub enum Command {
    Helo { domain: String },
    Ehlo { domain: String },
    MailFrom { address: String },
    RcptTo { address: String },
    Data,
    Quit,
}
#[derive(Clone)]
enum CommandKind {
    Helo,
    Ehlo,
    MailFrom,
    RcptTo,
    Data,
    Quit,
}

pub fn parse_command(input: &[u8]) -> anyhow::Result<Command> {
    match all_consuming(terminated(command_parser, multispace0))(input) {
        Ok((_, cmd)) => Ok(cmd),
        Err(err) => Err(anyhow!(
            "could not parse command from '{}', {}",
            String::from_utf8_lossy(input),
            err
        )),
    }
}

fn command_parser(input: &[u8]) -> IResult<&[u8], Command> {
    let (input, kind) = command_kind_parser(input)?;
    let (input, _) = multispace0(input)?;
    match kind {
        CommandKind::Helo => helo_parser(input),
        CommandKind::Ehlo => ehlo_parser(input),
        CommandKind::MailFrom => mail_from_parser(input),
        CommandKind::RcptTo => rcpt_to_parser(input),
        CommandKind::Data => data_parser(input),
        CommandKind::Quit => quit_parser(input),
    }
}

fn command_kind_parser(input: &[u8]) -> IResult<&[u8], CommandKind> {
    alt((
        value(CommandKind::Helo, tag_no_case("HELO")),
        value(CommandKind::Ehlo, tag_no_case("EHLO")),
        value(CommandKind::MailFrom, tag_no_case("MAIL FROM:")),
        value(CommandKind::RcptTo, tag_no_case("RCPT TO:")),
        value(CommandKind::Data, tag_no_case("DATA")),
        value(CommandKind::Quit, tag_no_case("QUIT")),
    ))(input)
}

fn helo_parser(input: &[u8]) -> IResult<&[u8], Command> {
    let (input, domain) = domain_parser(input)?;
    Ok((input, Command::Helo { domain }))
}

fn ehlo_parser(input: &[u8]) -> IResult<&[u8], Command> {
    let (input, domain) = domain_parser(input)?;
    Ok((input, Command::Ehlo { domain }))
}

fn mail_from_parser(input: &[u8]) -> IResult<&[u8], Command> {
    let (input, address) = delimited(tag("<"), address_parser, tag(">"))(input)?;
    Ok((input, Command::MailFrom { address }))
}

fn rcpt_to_parser(input: &[u8]) -> IResult<&[u8], Command> {
    let (input, address) = delimited(tag("<"), address_parser, tag(">"))(input)?;
    Ok((input, Command::RcptTo { address }))
}

fn quit_parser(input: &[u8]) -> IResult<&[u8], Command> {
    Ok((input, Command::Quit))
}
fn data_parser(input: &[u8]) -> IResult<&[u8], Command> {
    Ok((input, Command::Data))
}

fn domain_parser(input: &[u8]) -> IResult<&[u8], String> {
    let (input, domain) = take_while1(|c: u8| c.is_alphanum() || c == b'.')(input)?;
    Ok((input, String::from_utf8(domain.to_vec()).unwrap()))
}

fn address_parser(input: &[u8]) -> IResult<&[u8], String> {
    let (input, domain) = take_while1(|c: u8| c.is_alphanum() || c == b'.' || c == b'@')(input)?;
    Ok((input, String::from_utf8(domain.to_vec()).unwrap()))
}

#[cfg(test)]
mod test {
    use crate::protocol::{command_parser, Command};

    #[test]
    fn test_mail_from() -> anyhow::Result<()> {
        let (_, cmd) = command_parser(b"MAIL FROM:<alice@yo.dog>")?;
        assert_eq!(
            cmd,
            Command::MailFrom {
                address: "alice@yo.dog".to_owned()
            }
        );
        Ok(())
    }

    #[test]
    fn test_mail_from_with_space() -> anyhow::Result<()> {
        let (_, cmd) = command_parser(b"MAIL FROM:   <alice@yo.dog>")?;
        assert_eq!(
            cmd,
            Command::MailFrom {
                address: "alice@yo.dog".to_owned()
            }
        );
        Ok(())
    }
}
