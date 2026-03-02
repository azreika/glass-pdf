use std::fmt;

fn is_identifier_char(c: char) -> bool {
    if c.is_alphanumeric() {
        return true;
    }

    if c == '.' || c == '-' || c == '+' {
        return true;
    }

    return false;
}

#[derive(Debug, Copy, Clone)]
pub struct SrcLoc {
    pos: usize,
}

impl SrcLoc {
    pub fn new(pos: usize) -> Self {
        return SrcLoc {
            pos,
        }
    }
}

impl fmt::Display for SrcLoc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.pos)
    }
}


impl Tokenizer<'_> {
    fn lex_char(&mut self) -> char {
        let result = self.data[self.offset] as char;
        self.offset += 1;
        return result;
    }

    fn eat_char(&mut self, c: char) {
        assert!(self.peek_is(c));
        self.offset += 1;
    }

    fn eat_next(&mut self) -> u8 {
        let result = self.data[self.offset];
        self.offset += 1;
        return result;
    }

    fn peek_is(&self, c: char) -> bool {
        return self.data[self.offset] as char == c;
    }

    fn peek_is_nth(&self, n: usize, c: char) -> bool {
        return self.data[self.offset + n] as char == c;
    }

    fn peek(&self) -> char {
        return self.data[self.offset] as char;
    }

    fn push_token(&mut self, loc: SrcLoc, tok: Token) {
        self.tokens.push((loc, tok));
    }

    fn new(data: &Vec<u8>) -> Tokenizer<'_> {
        return Tokenizer {
            data,
            offset: 0,
            tokens: vec![],
        };
    }

    fn src_loc(&self) -> SrcLoc {
        return SrcLoc::new(self.offset);
    }

    fn lex_number(&mut self) -> i32 {
        let mut num = 0;
        while self.peek().is_numeric() {
            let dd = (self.eat_next() as char).to_digit(10).unwrap() as i32;
            num *= 10;
            num += dd;
        }
        return num;
    }

    fn eat_keyword(&mut self, word: String) {
        let mut idx = 0;
        while idx < word.len() {
            self.eat_char(word.chars().nth(idx).unwrap());
            idx += 1;
        }
        assert!(self.peek().is_whitespace());
    }

    fn eat_whitespace(&mut self) {
        while self.peek().is_whitespace() {
            self.eat_next();
        }
    }

    fn lex_identifier(&mut self) -> Token {
        let mut chars = vec![];
        while is_identifier_char(self.peek()) {
            chars.push(self.lex_char());
        }
        let str = chars.iter().collect();
        return Token::Identifier(str);
    }

    fn next_line_bytes(&mut self) -> Vec<u8> {
        let mut bytes = vec![];
        while !self.peek_is('\n') {
            bytes.push(self.eat_next());
        }
        self.eat_whitespace();
        return bytes;
    }

    fn run(&mut self) {
        assert!(self.tokens.is_empty());
        while self.offset < self.data.len() {
            let c = self.peek();
            let loc = self.src_loc();
            if c.is_whitespace() {
                self.eat_next();
            } else if c.is_numeric() {
                let num = self.lex_number();
                self.push_token(loc, Token::Number(num));
            } else if c == '-' {
                self.eat_char('-');
                let num = self.lex_number();
                self.push_token(loc, Token::Number(-1 * num));
            } else if c == '%' {
                self.eat_char('%');

                if self.peek() == '%' {
                    self.eat_char('%');
                    self.eat_keyword("EOF".to_string());
                    self.push_token(loc, Token::EOFKeyword);
                    continue;
                }

                self.push_token(loc, Token::Percent);


                let mut chars = vec![];
                while !self.peek_is('\n') {
                    chars.push(self.lex_char());
                }
                self.eat_char('\n');
                let str = chars.iter().collect();
                self.push_token(loc, Token::String(str));
            } else if c == '<' {
                self.eat_char('<');
                self.eat_whitespace();
                self.push_token(loc, Token::AngleStart);
                if !self.peek_is('<') {
                    let id = self.lex_identifier();
                    self.push_token(loc, id);
                } else {
                    self.eat_char('<');
                    self.push_token(loc, Token::AngleStart);
                }
            } else if c == '>' {
                self.eat_char('>');
                self.push_token(loc, Token::AngleEnd);
            } else if c == '/' {
                self.eat_char('/');
                self.push_token(loc, Token::ForwardSlash);
                let tok = self.lex_identifier();
                self.push_token(loc, tok);
            } else if c == 'o' {
                self.eat_keyword("obj".to_string());
                self.push_token(loc, Token::ObjKeyword);
            } else if c == 'R' {
                self.eat_keyword("R".to_string());
                self.push_token(loc, Token::RefKeyword);
            } else if c == 's' {
                if self.peek_is_nth(2, 'a') {
                    self.eat_keyword("startxref".to_string());
                    self.push_token(loc, Token::StartXRefKeyword);
                    continue;
                }
                self.eat_keyword("stream".to_string());
                self.push_token(loc, Token::StreamKeyword);

                self.eat_whitespace();

                loop {
                    let line_bytes = self.next_line_bytes();
                    match str::from_utf8(&line_bytes) {
                        Ok(v) => if v == "endstream".to_string() {
                            self.push_token(loc, Token::EndStreamKeyword);
                            break;
                        }
                        _ => {},
                    }
                    self.push_token(loc, Token::ByteStream(line_bytes))
                }
            } else if c == 'e' {
                self.eat_keyword("endobj".to_string());
                self.push_token(loc, Token::EndObjKeyword);
            } else if c == '[' {
                self.eat_char('[');
                self.push_token(loc, Token::LeftBracket);
            } else if c == ']' {
                self.eat_char(']');
                self.push_token(loc, Token::RightBracket);
            } else if c == '(' {
                self.eat_char('(');
                self.push_token(loc, Token::LeftParens);

                let mut chars = vec![];
                while !self.peek_is(')') {
                    chars.push(self.lex_char());
                }
                let str = chars.iter().collect();
                self.push_token(loc, Token::String(str));
                self.eat_char(')');
                self.push_token(loc, Token::RightParens);
            } else if c == 'n' {
                self.eat_keyword("n".to_string());
                self.push_token(loc, Token::NKeyword);
            } else if c == 'f' {
                self.eat_keyword("f".to_string());
                self.push_token(loc, Token::FKeyword);
            } else if c == 't' {
                self.eat_keyword("trailer".to_string());
                self.push_token(loc, Token::TrailerKeyword);
            } else if c == 'x' {
                self.eat_keyword("xref".to_string());
                self.push_token(loc, Token::XRefKeyword);
            } else {
                println!("FAILURE!!! {:?} @ loc{{{}}}/{}", c, self.offset, self.data.len());
                assert!(false);
            }
        }
    }
}

#[derive(Debug)]
struct Tokenizer<'a> {
    data: &'a Vec<u8>,
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
    let mut tokenizer = Tokenizer::new(data);
    tokenizer.run();
    return tokenizer.tokens;
}
