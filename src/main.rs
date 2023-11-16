use std::fs::{File, OpenOptions};
use std::io::{Write, Read};
use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

#[derive(Logos, Debug)]
enum Token {
    #[token("**")]
    Bold,

    #[token("*")]
    Italic,

    #[token("__")]
    Underline,

    #[regex(r"#( +)?")]
    TopHeader,

    #[regex(r"#{2,}( +)?")]
    LowerHeader,

    #[regex(r"( ?)\{#*.*\}", priority = 98)]
    Tag,

    #[token("\n")]
    RemovableNewline,

    #[token("\n\n")]
    ActiveNewline,

    #[regex(r"[^(\*\*)\*(__)#\n\r\t\f]")]
    Text,

    #[regex(r"[\-\*+] .+(\n)")]
    UnorderedList,
    
    // #[regex(r"[0-9]\. .+(\n)")]
    // OrderedList,

    #[regex(r"\[[^\[\]]+\]\([^\(\)]+\)", priority = 99)]
    Link,

    #[regex(r"(\n)?`{3}[^`]*`{3}(\n)?", priority = 100)]
    Codeblock,
}

#[derive(Parser, Clone, Debug)]
#[command(author, version, about, long_about = None)]
struct CliArgs {
    file: Option<PathBuf>,

    #[arg(short, long)]
    message: Option<String>,

    #[arg(short, long, value_name = "FILE")]
    destination: Option<PathBuf>,
}

#[derive(Debug, Default)]
struct State {
    top_header: bool,
    lower_header: bool,
    bold: bool,
    italic: bool,
    underline: bool,
    // double_strike: bool,
}

fn main() {
    let args = CliArgs::parse();
    let input = read_input(args.clone());

    let mut lex = Token::lexer(&input);
    let mut state = State::default();

    let mut res = Vec::<u8>::new();
    while let Some(r) = lex.next() {
        if let Ok(variant) = r {
            match variant {
                Token::Bold => res.extend_from_slice(bold(&mut state)),
                Token::Italic => res.extend_from_slice(italic(&mut state)),
                Token::Underline => res.extend_from_slice(underline(&mut state)),
                Token::TopHeader => res.extend_from_slice(top_header(&mut state)),
                // TODO: lower header formatting (font size)
                Token::LowerHeader => res.extend_from_slice(lower_header(&mut state)),
                Token::RemovableNewline => {
                    res.append(&mut new_line(&mut state, Token::RemovableNewline))
                },
                Token::ActiveNewline => {
                    res.append(&mut new_line(&mut state, Token::ActiveNewline))
                },
                Token::Tag => {}
                _ => res.extend_from_slice(lex.slice().as_bytes()),
            };
        }
    }
    res.push(b'\n');

    write_output(args, res.as_slice());
}

fn new_line<'a>(state: &mut State, variant: Token) -> Vec<u8> {
    let mut res = Vec::<u8>::new();

    if state.top_header {
        res.extend_from_slice(b"\x1BF\x1Bw0\x1BW0");

        state.top_header = false;
    }
    if state.lower_header {
        res.extend_from_slice(b"\x1Bq0");

        state.lower_header = false;
    }
    if matches!(variant, Token::ActiveNewline) {
        res.push(b'\n');
    } else {
        res.push(b' ')
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

fn top_header<'a>(state: &mut State) -> &[u8] {
    let res: &[u8];

    if !state.top_header {
        res = b"\n\x1BE\x1Bw1\x1BW1";

        state.top_header = true;
    } else {
        res = &[];
    }

    res
}

fn lower_header<'a>(state: &mut State) -> &[u8] {
    let res: &[u8];

    if !state.lower_header {
        res = b"\x1Bq1";

        state.lower_header = true;
    } else {
        res = &[];
    }

    res
}

fn read_input(args: CliArgs) -> String {
    let mut input: String = String::new();
    if let Some(filebuf) = args.file {
        let display_path = filebuf.display();
        let mut file = match File::open(&filebuf) {
            Ok(file) => file,
            Err(e) => panic!("Could not open {}: {}", display_path, e),
        };
        if let Err(e) = file.read_to_string(&mut input) {
            panic!("Cannot read from input {}: {}", display_path, e);
        }
    } else if let Some(string) = args.message {
        input = string;
    } else {
        panic!("Must provide an input source.");
    }

    input
}

fn write_output(args: CliArgs, slice: &[u8]) {
    let mut file: Box<dyn Write> = if let Some(filebuf) = args.destination {
        let display_path = filebuf.display();
        let local_file = match OpenOptions::new().write(true).create(true).open(&filebuf) {
            Ok(file) => file,
            Err(e) => panic!("Could not open {} for writing: {}", display_path, e),
        };
        Box::new(local_file)
    } else {
        Box::new(std::io::stdout())
    };

    file.write_all(slice).unwrap();
}
