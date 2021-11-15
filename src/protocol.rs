use anyhow::anyhow;
use nom::{
    bytes::complete::{tag, take_while1},
    character::complete::multispace1,
    combinator::{all_consuming, value},
    AsChar, IResult,
};

pub enum Command {
    Helo { name: String },
}
#[derive(Clone)]
enum CommandKind {
    Helo,
}

pub fn parse_command(input: &[u8]) -> anyhow::Result<Command> {
    match all_consuming(command_parser)(input) {
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
    }
}

fn command_kind_parser(input: &[u8]) -> IResult<&[u8], CommandKind> {
    value(CommandKind::Helo, tag("HELO"))(input)
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
