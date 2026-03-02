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

    fn new(data: &Vec<u8>) -> Tokenizer<'_> {
        return Tokenizer {
            data,
            offset: 0,
        };
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

    fn run(&mut self) -> Vec<Token> {
        let mut tokens = vec![];

        while self.offset < self.data.len() {
            let c = self.peek();
            if c.is_whitespace() {
                self.eat_next();
            } else if c.is_numeric() {
                let num = self.lex_number();
                tokens.push(Token::Number(num));
            } else if c == '-' {
                self.eat_char('-');
                let num = self.lex_number();
                tokens.push(Token::Number(-1 * num));
            } else if c == '%' {
                self.eat_char('%');

                if self.peek() == '%' {
                    self.eat_char('%');
                    self.eat_keyword("EOF".to_string());
                    tokens.push(Token::EOFKeyword);
                    continue;
                }

                tokens.push(Token::Percent);


                let mut chars = vec![];
                while !self.peek_is('\n') {
                    chars.push(self.lex_char());
                }
                self.eat_char('\n');
                let str = chars.iter().collect();
                tokens.push(Token::String(str));
            } else if c == '<' {
                self.eat_char('<');
                self.eat_whitespace();
                tokens.push(Token::AngleStart);
                if !self.peek_is('<') {
                    let id = self.lex_identifier();
                    tokens.push(id);
                } else {
                    self.eat_char('<');
                    tokens.push(Token::AngleStart);
                }
            } else if c == '>' {
                self.eat_char('>');
                tokens.push(Token::AngleEnd);
            } else if c == '/' {
                self.eat_char('/');
                tokens.push(Token::ForwardSlash);
                let tok = self.lex_identifier();
                tokens.push(tok);
            } else if c == 'o' {
                self.eat_keyword("obj".to_string());
                tokens.push(Token::ObjKeyword);
            } else if c == 'R' {
                self.eat_keyword("R".to_string());
                tokens.push(Token::RefKeyword);
            } else if c == 's' {
                if self.peek_is_nth(2, 'a') {
                    self.eat_keyword("startxref".to_string());
                    tokens.push(Token::StartXRefKeyword);
                    continue;
                }
                self.eat_keyword("stream".to_string());
                tokens.push(Token::StreamKeyword);

                self.eat_whitespace();

                loop {
                    let line_bytes = self.next_line_bytes();
                    match str::from_utf8(&line_bytes) {
                        Ok(v) => if v == "endstream".to_string() {
                            tokens.push(Token::EndStreamKeyword);
                            break;
                        }
                        _ => {},
                    }
                    tokens.push(Token::ByteStream(line_bytes))
                }
            } else if c == 'e' {
                self.eat_keyword("endobj".to_string());
                tokens.push(Token::EndObjKeyword);
            } else if c == '[' {
                self.eat_char('[');
                tokens.push(Token::LeftBracket);
            } else if c == ']' {
                self.eat_char(']');
                tokens.push(Token::RightBracket);
            } else if c == '(' {
                self.eat_char('(');
                tokens.push(Token::LeftParens);

                let mut chars = vec![];
                while !self.peek_is(')') {
                    chars.push(self.lex_char());
                }
                let str = chars.iter().collect();
                tokens.push(Token::String(str));
                self.eat_char(')');
                tokens.push(Token::RightParens);
            } else if c == 'n' {
                self.eat_keyword("n".to_string());
                tokens.push(Token::NKeyword);
            } else if c == 'f' {
                self.eat_keyword("f".to_string());
                tokens.push(Token::FKeyword);
            } else if c == 't' {
                self.eat_keyword("trailer".to_string());
                tokens.push(Token::TrailerKeyword);
            } else if c == 'x' {
                self.eat_keyword("xref".to_string());
                tokens.push(Token::XRefKeyword);
            } else {
                println!("{:?}", tokens);
                println!("FAILURE!!! {:?} @ loc{{{}}}/{}", c, self.offset, self.data.len());
                assert!(false);
            }
        }

        return tokens;
    }
}



#[derive(Debug)]
struct Tokenizer<'a> {
    data: &'a Vec<u8>,
    offset: usize,
}

#[derive(Debug, PartialEq)]
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

pub fn tokenize_pdf(data: &Vec<u8>) -> Vec<Token> {
    let mut tokenizer = Tokenizer::new(data);
    return tokenizer.run();
}
