use crate::tokenizer::{Tokenizer, is_identifier_char};
use crate::src_loc::SrcLoc;
use std::fmt;

impl Tokenizer<Token> for PdfTokenizer {
    fn token_from_word(&self, word: &str) -> Token {
        return match word {
            "xref" => Token::XRefKeyword,
            "trailer" => Token::TrailerKeyword,
            "f" => Token::FKeyword,
            "n" => Token::NKeyword,
            "endobj" => Token::EndObjKeyword,
            "R" => Token::RefKeyword,
            "obj" => Token::ObjKeyword,
            "startxref" => Token::StartXRefKeyword,
            "stream" => Token::StreamKeyword,

            "[" => Token::LeftBracket,
            "]" => Token::RightBracket,
            ">" => Token::AngleEnd,
            "/" => Token::ForwardSlash,
            _ => {
                println!("FAILURE!!! {word} @ loc{{{}}}/{}", self.offset, self.data.len());
                panic!();
            }
        }
    }

    fn peek_u8(&self) -> u8 {
        return self.data[self.offset];
    }

    fn step_ahead(&mut self) {
        self.offset += 1;
    }
}

impl PdfTokenizer {
    fn eat_char(&mut self, c: char) {
        assert!(self.peek_is(c));
        self.offset += 1;
    }

    fn push_token(&mut self, loc: SrcLoc, tok: Token) {
        self.tokens.push((loc, tok));
    }

    fn new(data: Vec<u8>) -> PdfTokenizer {
        return PdfTokenizer {
            data,
            offset: 0,
            tokens: vec![],
        };
    }

    fn src_loc(&self) -> SrcLoc {
        return SrcLoc::new(self.offset);
    }

    fn lex_number(&mut self) -> i32 {
        let negative = self.peek_is('-');
        if negative {
            self.eat_char('-');
        }

        let mut num = 0;
        while self.peek().is_numeric() {
            num *= 10;
            num += self.lex_char().to_digit(10).unwrap() as i32;
        }
        return if negative { -num } else { num };
    }

    fn eat_whitespace(&mut self) {
        while self.peek().is_whitespace() {
            self.lex_char();
        }
    }

    fn lex_until(&mut self, final_char: char) -> String {
        let mut chars = vec![];
        while !self.peek_is(final_char) {
            chars.push(self.lex_char());
        }
        return chars.iter().collect();
    }

    fn has_bytes(&self) -> bool {
        return self.offset < self.data.len();
    }

    fn run(&mut self) {
        assert!(self.tokens.is_empty());

        while self.has_bytes() {
            let c = self.peek();
            let loc = self.src_loc();
            if c.is_whitespace() {
                self.lex_char();
            } else if c.is_numeric() || c == '-' {
                let num = self.lex_number();
                self.push_token(loc, Token::Number(num));
            } else if c == '%' {
                self.eat_char('%');
                if self.peek() == '%' {
                    self.eat_char('%');
                    let word = self.lex_word();
                    assert_eq!(word, "EOF");
                    self.push_token(loc, Token::EOFKeyword);
                    continue;
                }
                self.push_token(loc, Token::Percent);
                let str = self.lex_until('\n');
                self.eat_char('\n');
                self.push_token(loc, Token::String(str));
            } else if c == '<' {
                self.eat_char('<');
                self.eat_whitespace();
                self.push_token(loc, Token::AngleStart);
                if !self.peek_is('<') {
                    let id_word = self.lex_word();
                    let id = Token::Identifier(id_word);
                    self.push_token(loc, id);
                } else {
                    self.eat_char('<');
                    self.push_token(loc, Token::AngleStart);
                }
            } else if c == '(' {
                self.eat_char('(');
                self.push_token(loc, Token::LeftParens);

                let mut chars = vec![];
                // TODO: add depth params
                while !self.peek_is(')') {
                    chars.push(self.lex_char());
                }
                let str = chars.iter().collect();
                self.push_token(loc, Token::String(str));
                self.eat_char(')');
                self.push_token(loc, Token::RightParens);
            } else {
                let word = self.lex_word();
                let tok = self.token_from_word(&word);
                self.push_token(loc, tok.clone());

                if matches!(tok, Token::StreamKeyword) {
                    self.lex_stream_body(loc);
                } else if matches!(tok, Token::ForwardSlash) {
                    let id_word = self.lex_word();
                    self.push_token(loc, Token::Identifier(id_word));
                }
            }
        }
    }

    // Lex the next word or delimiter in the byte sequence
    fn lex_word(&mut self) -> String {
        if !is_identifier_char(self.peek()) {
            return self.lex_char().to_string();
        }

        let mut chars = vec![];
        while is_identifier_char(self.peek()) {
            chars.push(self.lex_char());
        }
        let str = chars.iter().collect();
        return str;
    }

    fn lex_stream_body(&mut self, loc: SrcLoc) {
        self.eat_char('\n');
        let start = self.offset;

        // Scan for "endstream" marker
        while self.offset < self.data.len() {
            if self.data[self.offset..].starts_with(b"endstream") {
                break;
            }
            self.offset += 1;
        }

        // Trim trailing \n or \r\n before endstream
        let mut end = self.offset;
        if end > start && self.data[end - 1] == b'\n' { end -= 1; }
        if end > start && self.data[end - 1] == b'\r' { end -= 1; }

        self.push_token(loc, Token::ByteStream(self.data[start..end].to_vec()));

        // Consume "endstream"
        self.offset += "endstream".len();
        self.push_token(loc, Token::EndStreamKeyword);
    }
}

#[derive(Debug)]
struct PdfTokenizer {
    data: Vec<u8>,
    offset: usize,
    tokens: Vec<(SrcLoc, Token)>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    Number(i32),
    Identifier(String),
    String(String),
    ByteStream(Vec<u8>),

    // Special characters
    AngleStart,
    AngleEnd,
    ForwardSlash,
    LeftBracket,
    RightBracket,
    LeftParens,
    RightParens,
    Percent,

    // Keywords
    ObjKeyword,
    EndObjKeyword,
    StreamKeyword,
    EndStreamKeyword,
    XRefKeyword,
    StartXRefKeyword,
    TrailerKeyword,
    RefKeyword,
    FKeyword,
    NKeyword,
    EOFKeyword,
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut kw = |x| { return write!(f, "{}", x) };
        match self {
            Token::Number(v) => write!(f, "{}", v),
            Token::Identifier(str) => write!(f, "{}", str),
            Token::String(str) => write!(f, "{}", str),
            Token::ByteStream(_) => write!(f, "<bytes...>"),

            Token::AngleStart => kw("<"),
            Token::AngleEnd => kw(">"),
            Token::ForwardSlash => kw("/"),
            Token::LeftBracket => kw("["),
            Token::RightBracket => kw("]"),
            Token::LeftParens => kw("("),
            Token::RightParens => kw(")"),
            Token::Percent => kw("%"),

            Token::ObjKeyword => kw("obj"),
            Token::EndObjKeyword => kw("endobj"),
            Token::StreamKeyword => kw("stream"),
            Token::EndStreamKeyword => kw("endstream"),
            Token::XRefKeyword => kw("xref"),
            Token::StartXRefKeyword => kw("startxref"),
            Token::TrailerKeyword => kw("trailer"),
            Token::RefKeyword => kw("R"),
            Token::FKeyword => kw("f"),
            Token::NKeyword => kw("n"),
            Token::EOFKeyword => kw("EOF"),
        }
    }
}

pub fn tokenize_pdf(data: &Vec<u8>) -> Vec<(SrcLoc,Token)> {
    let mut tokenizer = PdfTokenizer::new(data.clone());
    tokenizer.run();
    return tokenizer.tokens;
}
