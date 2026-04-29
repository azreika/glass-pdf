
#[derive(Debug)]
pub struct ContentAst {
    txt: String,
}

enum ContentToken {
    SaveGraphicsState,
    RestoreGraphicsState,
    Number(i32),
    RectKeyword,
    WKeyword,
    WStarKeyword,
    NKeyword,
    Identifier(String),
    CsStroke,
    CsNoStroke,
    EndCsStroke,
    EndCsNoStroke,
    f,
}

struct ContentTokenizer {
    data: String,
    offset: usize,
}

impl ContentTokenizer {
    fn peek(&self) -> char {
        return self.data.chars().nth(self.offset).unwrap();
    }

    fn eat_whitespace(&mut self) {
        while self.peek().is_whitespace() {
            self.eat_next();
        }
    }

    fn eat_next(&mut self) -> char {
        let result =self.data.chars().nth(self.offset).unwrap();
        self.offset += 1;
        return result;
    }

    fn lex_number(&mut self) -> i32 {
        let mut num = 0;
        while self.peek().is_numeric() {
            let dd = self.eat_next().to_digit(10).unwrap() as i32;
            num *= 10;
            num += dd;
        }
        return num;
    }

    fn peek_is(&self, c: char) -> bool {
        return self.peek() == c;
    }

    fn eat_char(&mut self, c: char) {
        assert!(self.peek_is(c));
        self.offset += 1;
    }

    fn eat_keyword(&mut self, word: String) {
        let mut idx = 0;
        while idx < word.len() {
            self.eat_char(word.chars().nth(idx).unwrap());
            idx += 1;
        }
        assert!(self.peek().is_whitespace());
    }

    fn lex_identifier(&mut self) -> ContentToken {
        let mut chars = vec![];
        while is_identifier_char(self.peek()) {
            chars.push(self.lex_char());
        }
        let str = chars.iter().collect();
        return ContentToken::Identifier(str);
    }

    fn lex_char(&mut self) -> char {
        let result = self.peek();
        self.offset += 1;
        return result;
    }
}

  fn is_identifier_char(c: char) -> bool {
    if c.is_alphanumeric() {
            return true;
        }

        if c == '.' || c == '-' || c == '+' {
            return true;
        }

        return false;
    }

fn tokenize_stream(str: String) -> Vec<ContentToken> {
    let mut vv = vec![];

    let mut tokenizer = ContentTokenizer { data: str, offset: 0 };
    while tokenizer.offset < tokenizer.data.len() {
        tokenizer.eat_whitespace();
        let cc = tokenizer.peek();
        if cc == 'q' {
            tokenizer.eat_next();
            vv.push(ContentToken::SaveGraphicsState);
        } else if cc == 'Q' {
            tokenizer.eat_next();
            vv.push(ContentToken::RestoreGraphicsState);
        } else if cc.is_numeric() {
            let num = tokenizer.lex_number();
            vv.push(ContentToken::Number(num));
        } else if cc == 'r' {
            tokenizer.eat_keyword("re".to_string());
            vv.push(ContentToken::RectKeyword);
        } else if cc == 'W' {
            tokenizer.eat_char('W');
            if tokenizer.peek() == '*' {
                tokenizer.eat_char('*');
                vv.push(ContentToken::WStarKeyword);
            } else {
                vv.push(ContentToken::WKeyword);
            }
        } else if cc == 'n' {
            tokenizer.eat_char('n');
            vv.push(ContentToken::NKeyword);
        } else if cc == '/' {
            tokenizer.eat_char('/');
            let id = tokenizer.lex_identifier();
            vv.push(id);
        } else if cc == 'c' {
            tokenizer.eat_next();
            assert!(tokenizer.peek() == 's');
            tokenizer.eat_next();
            vv.push(ContentToken::CsStroke);
        } else if cc == 'C' {
            tokenizer.eat_next();
            assert!(tokenizer.peek() == 'S');
            tokenizer.eat_next();
            vv.push(ContentToken::CsStroke);
        } else if cc == 's' {
            tokenizer.eat_next();
            tokenizer.eat_char('c');
            vv.push(ContentToken::EndCsStroke);
        } else if cc == 'S' {
            tokenizer.eat_next();
            tokenizer.eat_char('C');
            vv.push(ContentToken::EndCsNoStroke);
        } else {
            println!("missed char: {}", tokenizer.peek());
            println!("{}", &tokenizer.data[tokenizer.offset..tokenizer.data.len()]);
            panic!();
        }
    }

    return vv;
}

pub fn parse_stream(result: String) -> ContentAst {
    let tokens = tokenize_stream(result);
    return parse_program(&tokens);
}

fn parse_program(result: &Vec<ContentToken>) -> ContentAst {
    return ContentAst { txt: "haha".to_string() };
}
