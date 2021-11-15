use anyhow::anyhow;
use nom::{
    branch::alt,
    bytes::complete::{tag, take_while1},
    character::complete::{multispace0, multispace1},
    combinator::{all_consuming, value},
    sequence::terminated,
    AsChar, IResult,
};

pub enum Command {
    Helo { name: String },
    Quit,
}
#[derive(Clone)]
enum CommandKind {
    Helo,
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
    let (input, _) = multispace1(input)?;
    match kind {
        CommandKind::Helo => helo_parser(input),
        CommandKind::Quit => quit_parser(input),
    }
}

fn command_kind_parser(input: &[u8]) -> IResult<&[u8], CommandKind> {
    alt((
        value(CommandKind::Helo, tag("HELO")),
        value(CommandKind::Quit, tag("QUIT")),
    ))(input)
}

fn helo_parser(input: &[u8]) -> IResult<&[u8], Command> {
    let (input, name) = take_while1(|c: u8| c.is_alphanum() || c == b'.')(input)?;
    Ok((
        input,
        Command::Helo {
            name: String::from_utf8(name.to_vec()).unwrap(),
        },
    ))
}

fn quit_parser(input: &[u8]) -> IResult<&[u8], Command> {
    Ok((input, Command::Quit))
}
