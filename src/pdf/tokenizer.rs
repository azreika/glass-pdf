use crate::tokenizer::Tokenizer;
use crate::pdf::ast::SrcLoc;
use std::fmt;

#[derive(Debug, PartialEq, Clone)]
pub enum PdfToken {
    Number(f32),
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

    BooleanTrue,
    BooleanFalse,
    Null,
}

impl fmt::Display for PdfToken {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut kw = |x| { return write!(f, "{}", x) };
        match self {
            PdfToken::Number(v) => write!(f, "{}", v),
            PdfToken::Identifier(str) => write!(f, "{}", str),
            PdfToken::String(str) => write!(f, "{}", str),
            PdfToken::ByteStream(_) => write!(f, "<bytes...>"),

            PdfToken::AngleStart => kw("<"),
            PdfToken::AngleEnd => kw(">"),
            PdfToken::ForwardSlash => kw("/"),
            PdfToken::LeftBracket => kw("["),
            PdfToken::RightBracket => kw("]"),
            PdfToken::LeftParens => kw("("),
            PdfToken::RightParens => kw(")"),
            PdfToken::Percent => kw("%"),

            PdfToken::ObjKeyword => kw("obj"),
            PdfToken::EndObjKeyword => kw("endobj"),
            PdfToken::StreamKeyword => kw("stream"),
            PdfToken::EndStreamKeyword => kw("endstream"),
            PdfToken::XRefKeyword => kw("xref"),
            PdfToken::StartXRefKeyword => kw("startxref"),
            PdfToken::TrailerKeyword => kw("trailer"),
            PdfToken::RefKeyword => kw("R"),
            PdfToken::FKeyword => kw("f"),
            PdfToken::NKeyword => kw("n"),
            PdfToken::EOFKeyword => kw("EOF"),
            PdfToken::BooleanTrue => kw("true"),
            PdfToken::BooleanFalse => kw("false"),
            PdfToken::Null => kw("null"),
        }
    }
}

#[derive(Debug)]
struct PdfTokenizer {
    data: Vec<u8>,
    offset: usize,
    tokens: Vec<(SrcLoc, PdfToken)>,
}

impl Tokenizer<PdfToken> for PdfTokenizer {
    fn token_from_word(&self, word: &str) -> PdfToken {
        return match word {
            "xref" => PdfToken::XRefKeyword,
            "trailer" => PdfToken::TrailerKeyword,
            "f" => PdfToken::FKeyword,
            "n" => PdfToken::NKeyword,
            "endobj" => PdfToken::EndObjKeyword,
            "R" => PdfToken::RefKeyword,
            "obj" => PdfToken::ObjKeyword,
            "startxref" => PdfToken::StartXRefKeyword,
            "stream" => PdfToken::StreamKeyword,

            "[" => PdfToken::LeftBracket,
            "]" => PdfToken::RightBracket,
            ">" => PdfToken::AngleEnd,
            "/" => PdfToken::ForwardSlash,
            "true" => PdfToken::BooleanTrue,
            "false" => PdfToken::BooleanFalse,
            "null" => PdfToken::Null,
            _ => {
                println!("FAILURE!!! `{word}` @ loc{{{}}}/{}", self.offset, self.data.len());
                panic!();
            }
        }
    }

    fn peek_u8(&self) -> u8 {
        return self.data[self.offset];
    }

    fn has_next(&self) -> bool {
        return self.offset < self.data.len();
    }

    fn step_ahead(&mut self) {
        self.offset += 1;
    }
}

impl PdfTokenizer {
    fn push_token(&mut self, loc: SrcLoc, tok: PdfToken) {
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
                let num = self.parse_next_number();
                self.push_token(loc, PdfToken::Number(num));
            } else if c == '%' {
                self.eat_char('%');
                if self.peek() == '%' {
                    self.eat_char('%');
                    let word = self.lex_word();
                    assert_eq!(word, "EOF");
                    self.push_token(loc, PdfToken::EOFKeyword);
                    continue;
                }
                self.push_token(loc, PdfToken::Percent);
                let str = self.lex_until('\n');
                self.eat_char('\n');
                self.push_token(loc, PdfToken::String(str));
            } else if c == '<' {
                self.eat_char('<');
                self.eat_whitespace();
                self.push_token(loc, PdfToken::AngleStart);
                if !self.peek_is('<') {
                    let id_word = self.lex_word();
                    let id = PdfToken::Identifier(id_word);
                    self.push_token(loc, id);
                } else {
                    self.eat_char('<');
                    self.push_token(loc, PdfToken::AngleStart);
                }
            } else if c == '(' {
                self.eat_char('(');
                self.push_token(loc, PdfToken::LeftParens);

                let mut chars = vec![];
                // TODO: add depth params
                let mut depth = 1;
                while depth > 0 && self.offset < self.data.len() {
                    let mm = self.lex_char();
                    match mm {
                        '(' => { depth += 1; chars.push(mm); },
                        ')' => {
                            depth -= 1;
                            if depth > 0 { chars.push(mm) };
                        },
                        '\\' => {
                            chars.push(mm);
                            if self.offset < self.data.len() {
                                chars.push(self.lex_char()); // consume \(, \), \\, \n, etc.
                            }
                        },
                        _ => chars.push(mm),
                    }
                }
                let str = chars.iter().collect();
                self.push_token(loc, PdfToken::String(str));
                self.push_token(loc, PdfToken::RightParens);
            } else {
                let word = self.lex_word();
                let tok = self.token_from_word(&word);
                self.push_token(loc, tok.clone());

                if matches!(tok, PdfToken::StreamKeyword) {
                    self.lex_stream_body(loc);
                } else if matches!(tok, PdfToken::ForwardSlash) {
                    let id_word = self.lex_word();
                    self.push_token(loc, PdfToken::Identifier(id_word));
                }
            }
        }
    }

    fn eat_newline(&mut self) {
        if self.peek_is('\r') {
            self.eat_char('\r');
        }
        self.eat_char('\n');
    }

    fn lex_stream_body(&mut self, loc: SrcLoc) {
        self.eat_newline();
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

        self.push_token(loc, PdfToken::ByteStream(self.data[start..end].to_vec()));

        // Consume "endstream"
        self.offset += "endstream".len();
        self.push_token(loc, PdfToken::EndStreamKeyword);
    }
}

pub fn tokenize_pdf(data: &Vec<u8>) -> Vec<(SrcLoc,PdfToken)> {
    let mut tokenizer = PdfTokenizer::new(data.clone());
    tokenizer.run();
    return tokenizer.tokens;
}
