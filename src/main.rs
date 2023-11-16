use std::io::Write;
use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

#[derive(Logos, Debug)]
// #[logos(skip r"[ \t\f]+")]
enum Token {
    #[token("**")]
    Bold,

    #[token("*")]
    Italic,

    #[token("__")]
    Underline,

    #[regex("(#( +)?)+")]
    Header,

    #[token("\n")]
    RemovableNewline,

    #[token("\n\n")]
    ActiveNewline,

    #[regex(r"[A-Za-z0-9. ]+")]
    Text,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    string: Option<String>,

    #[arg(short, long, value_name = "FILE")]
    file: Option<PathBuf>,

    #[arg(short, long, value_name = "FILE")]
    destination: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct State {
    double_high: bool,
    double_wide: bool,
    bold: bool,
    italic: bool,
    underline: bool,
    // double_strike: bool,
}

fn main() {
    let mut state = State::default();

    let input =
        "# Lexing Test\n\nHello, **this** is a sample *markdown* string.\n This should be on the same line.\n\n***This should be bold and italic.***\n\n__This should be underlined.__";
    let mut lex = Token::lexer(input);

    let mut resultant = Vec::<u8>::new();

    for _ in 0..10000 {
        if let Some(Ok(variant)) = lex.next() {
            match variant {
                Token::Bold => resultant.extend_from_slice(bold(&mut state)),
                Token::Italic => resultant.extend_from_slice(italic(&mut state)),
                Token::Underline => resultant.extend_from_slice(underline(&mut state)),
                Token::Header => resultant.extend_from_slice(header(&mut state)),
                Token::RemovableNewline => {
                    resultant.append(&mut new_line(&mut state, Token::RemovableNewline))
                }
                Token::ActiveNewline => {
                    resultant.append(&mut new_line(&mut state, Token::ActiveNewline))
                }
                Token::Text => resultant.extend_from_slice(lex.slice().as_bytes()),
            };
        }
    }

    let mut stdout = std::io::stdout();
    stdout
        .write_all(resultant.as_slice())
        .expect("Unable to write to stdout");
    stdout.flush().expect("Unable to flush buffer");
    println!();
}

fn new_line<'a>(state: &mut State, variant: Token) -> Vec<u8> {
    let mut res = Vec::<u8>::new();

    if state.double_high && state.double_wide {
        res.extend_from_slice(b"\x1Bw0\x1BW0");

        state.double_high = false;
        state.double_wide = false;
    }
    if matches!(variant, Token::ActiveNewline) {
        res.extend_from_slice(b"\n");
    }

    res
}

fn bold<'a>(state: &mut State) -> &[u8] {
    let res;
    if !state.bold {
        res = b"\x1BE";
    } else {
        res = b"\x1BF";
    }

    state.bold = !state.bold;

    res
}

fn italic<'a>(state: &mut State) -> &[u8] {
    let res;
    if !state.italic {
        res = b"\x1B4";
    } else {
        res = b"\x1B5";
    }

    state.italic = !state.italic;

    res
}

fn underline<'a>(state: &mut State) -> &[u8] {
    let res;
    if !state.underline {
        res = b"\x1B-1";
    } else {
        res = b"\x1B-0";
    }

    state.underline = !state.underline;

    res
}

fn header<'a>(state: &mut State) -> &[u8] {
    let res: &[u8];

    if !(state.double_high && state.double_wide) {
        res = b"\n\x1Bw1\x1BW1";

        state.double_high = true;
        state.double_wide = true;
    } else {
        res = &[];
    }

    res
}
