use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::PathBuf;

use clap::Parser;
use logos::Logos;

macro_rules! def_wrap_env {
    ($name:ident, $fname:ident, $pre:tt, $post:tt) => {
        fn $name<'a>(state: &mut State) -> &'a [u8] {
            let res = if !state.$fname { $pre } else { $post };
            state.$fname = !state.$fname;
            return res;
        }
    };
}
macro_rules! def_open_env {
    ($name:ident, $fname:ident, $pre:tt) => {
        fn $name<'a>(state: &mut State) -> &'a [u8] {
            if !state.$fname {
                state.$fname = true;
                $pre
            } else {
                &[]
            }
        }
    };
}
macro_rules! def_close_env {
    ($name:ident, $fname:ident, $post:tt) => {
        fn $name<'a>(state: &mut State) -> &'a [u8] {
            if !state.$fname {
                &[]
            } else {
                state.$fname = false;
                $post
            }
        }
    };
}

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

    #[token("\n{2,}")]
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
}

def_wrap_env!(wrap_bold, bold, b"\x1BE", b"\x1BF");
def_wrap_env!(wrap_italic, italic, b"\x1B4", b"\x1B5");
def_wrap_env!(wrap_underline, underline, b"\x1B-1", b"\x1B-0");
def_open_env!(open_top_header, top_header, b"\n\n\x1BE\x1Bw1\x1BW1");
def_close_env!(close_top_header, top_header, b"\x1BF\x1Bw0\x1BW0\n");
def_open_env!(open_lower_header, lower_header, b"\n\n\x1Bw1");
def_close_env!(close_lower_header, lower_header, b"\x1Bw0\n");

fn main() {
    let args = CliArgs::parse();
    let input = read_input(args.clone());

    let res = transpile_markdown(&input);

    write_output(args, res.as_slice());
}

fn transpile_markdown(input: &str) -> Vec<u8> {
    let mut lex = Token::lexer(&input);
    let mut state = State::default();

    let mut res = Vec::<u8>::new();
    while let Some(r) = lex.next() {
        if let Ok(variant) = r {
            match variant {
                Token::Bold => res.extend_from_slice(wrap_bold(&mut state)),
                Token::Italic => res.extend_from_slice(wrap_italic(&mut state)),
                Token::Underline => res.extend_from_slice(wrap_underline(&mut state)),
                Token::TopHeader => res.extend_from_slice(open_top_header(&mut state)),
                // TODO: lower header formatting (font size)
                Token::LowerHeader => res.extend_from_slice(open_lower_header(&mut state)),
                Token::RemovableNewline => {
                    res.append(&mut new_line(&mut state, Token::RemovableNewline))
                }
                Token::ActiveNewline => res.append(&mut new_line(&mut state, Token::ActiveNewline)),
                Token::Tag => {}
                _ => res.extend_from_slice(lex.slice().as_bytes()),
            };
        }
    }
    res.push(b'\n');

    res
}

fn new_line(state: &mut State, variant: Token) -> Vec<u8> {
    let mut res = Vec::<u8>::new();

    res.extend_from_slice(close_top_header(state));
    res.extend_from_slice(close_lower_header(state));

    if matches!(variant, Token::ActiveNewline) {
        res.push(b'\n');
    } else {
        res.push(b' ')
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_line_removes_single_newlines() {
        let mut state = State::default();

        let res = new_line(&mut state, Token::RemovableNewline);

        assert_eq!(res, &[b' ']);
    }

    #[test]
    fn new_line_collapses_multiple_newlines() {
        let mut state = State::default();

        let res = new_line(&mut state, Token::ActiveNewline);

        assert_eq!(res, &[b'\n']);
    }

    #[test]
    fn bold_transpiles() {
        let input = "**bold text**";
        let expected_output = b"\x1BEbold text\x1BF\n";
        let res = transpile_markdown(&input);

        assert_eq!(res.as_slice(), expected_output)
    }

    #[test]
    fn italic_transpiles() {
        let input = "*italic text*";
        let expected_output = b"\x1B4italic text\x1B5\n";
        let res = transpile_markdown(&input);

        assert_eq!(res.as_slice(), expected_output)
    }

    #[test]
    fn underlined_transpiles() {
        let input = "__underlined text__";
        let expected_output = b"\x1B-1underlined text\x1B-0\n";
        let res = transpile_markdown(&input);

        assert_eq!(res.as_slice(), expected_output)
    }

    #[test]
    fn top_header_transpiles() {
        let input = "# Header text\n";
        let expected_output = b"\n\n\x1BE\x1Bw1\x1BW1Header text\x1BF\x1Bw0\x1BW0\n\n";
        let res = transpile_markdown(&input);

        assert_eq!(res.as_slice(), expected_output)
    }

    #[test]
    fn lower_header_transpiles() {
        let input = "## Header text\n";
        let expected_output = b"\n\n\x1Bw1Header text\x1Bw0\n";
        let res = transpile_markdown(&input);

        assert_eq!(res.as_slice(), expected_output)
    }
}
