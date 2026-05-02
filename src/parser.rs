use crate::pdf_tokenizer::Token;
use crate::src_loc::SrcLoc;
use crate::ast::{Pdf,Block,Value};
use std::collections::HashMap;

struct Parser<'a> {
    tokens: &'a Vec<(SrcLoc, Token)>,
    offset: usize,
}

fn is_string(tok: &Token) -> bool {
    return match tok {
        Token::String(_) => true,
        _ => false,
    }
}

fn is_identifier(tok: &Token) -> bool {
    return match tok {
        Token::Identifier(_) => true,
        _ => false,
    }
}

fn is_num(tok: &Token) -> bool {
    return match tok {
        Token::Number(_) => true,
        _ => false,
    }
}

fn is_bytestream(tok: &Token) -> bool {
    return match tok {
        Token::ByteStream(_) => true,
        _ => false,
    }
}

fn get_id(tok: &Token) -> String {
    return match tok {
        Token::Identifier(x) => x.to_string(),
        _ => {
            assert!(false);
            "".to_string()
        }
    }
}

fn get_str(tok: &Token) -> String {
    return match tok {
        Token::String(x) => x.to_string(),
        _ => {
            assert!(false);
            "".to_string()
        }
    }
}


fn get_num(tok: &Token) -> i32 {
    return match tok {
        Token::Number(v) => *v,
        _ => {
            assert!(false);
            0
        }
    };
}

fn get_bytes(tok: &Token) -> Vec<u8> {
    return match tok {
        Token::ByteStream(bytes) => bytes.clone().to_vec(),
        _ => {
            assert!(false);
            vec![]
        }
    }
}

pub fn parse_tokens(tokens: &Vec<(SrcLoc, Token)>) -> Pdf {
    let mut parser = Parser { tokens, offset: 0 };
    return parser.run_parser();
}

impl Parser<'_> {
    fn run_parser(&mut self) -> Pdf {
        let mut blocks = vec![];
        let mut start_xref = 0;
        let mut xref = vec![];
        while self.offset < self.tokens.len() {
            let tok = self.peek();
            match tok {
                Token::Percent => {
                    self.eat_next_token();
                    assert!(is_string(&self.peek()));
                    self.eat_next_token();
                },
                Token::Number(_) => {
                    let obj = self.eat_object();
                    blocks.push(obj);
                },
                Token::XRefKeyword => {
                    self.eat_next_token();
                    let v1 = self.eat_next_token();
                    assert!(is_num(&v1));
                    let v2 = self.eat_next_token();
                    assert!(is_num(&v2));

                    while self.peek() != Token::TrailerKeyword {
                        let v1 = self.eat_next_token();
                        assert!(is_num(&v1));
                        let offset = get_num(&v1);
                        let v2 = self.eat_next_token();
                        assert!(is_num(&v2));
                        let kk = self.eat_next_token();
                        assert!(kk == Token::FKeyword || kk == Token::NKeyword);
                        xref.push(SrcLoc::new(offset as usize));
                    }
                },
                Token::TrailerKeyword => {
                    self.eat_next_token();
                    let result = self.eat_dictionary();
                    blocks.push(Block::Trailer(result));
                    self.eat_expected(Token::StartXRefKeyword);
                    let v1 = self.eat_next_token();
                    assert!(is_num(&v1));
                    assert_eq!(start_xref, 0);
                    start_xref = get_num(&v1);
                    self.eat_expected(Token::EOFKeyword);
                },
                _ => {
                    println!("{:?}", blocks);
                    println!("{:?}", tok);
                    assert!(false);
                },
            }

        }

        blocks.push(Block::XRefTable(xref, SrcLoc::new(start_xref as usize)));

        return Pdf { blocks };
    }

    fn peek_is_dict(&self) -> bool {
        if self.peek() != Token::AngleStart {
            return false;
        }
        return self.peek_nth(1) == Token::AngleStart;
    }

    fn eat_bytestream(&mut self) -> Vec<u8> {
        assert!(self.eat_next_token() == Token::StreamKeyword);
                        assert!(is_bytestream(&self.peek()));
                        let mut bytes = vec![];
        let mut bytes_tok = self.eat_next_token();
        while is_bytestream(&bytes_tok) {
            for bb in get_bytes(&bytes_tok) {
                bytes.push(bb);
            }
            bytes_tok = self.eat_next_token();
        }
        assert!(bytes_tok == Token::EndStreamKeyword);
        return bytes;
    }

    fn src_loc(&self) -> SrcLoc {
        return self.tokens[self.offset].0;
    }

    fn expect_end_object(&mut self) {
        let token = self.eat_next_token();
        if token != Token::EndObjKeyword {
            println!("Expected end of object, but got `{}`.", token);
            self.throw_error();
        }
    }

    fn throw_error(&self) {
        panic!("failed");
    }

    fn eat_dictionary(&mut self) -> HashMap<String,Value> {
        let mut result = HashMap::new();

        self.eat_expected(Token::AngleStart);
        self.eat_expected(Token::AngleStart);

        loop {
            if self.peek() == Token::AngleEnd {
                break;
            }
            assert!(self.eat_next_token() == Token::ForwardSlash);
            let tok = self.eat_next_token();
            assert!(is_identifier(&tok));

            let val = self.eat_value();
            result.insert(get_id(&tok), val);
        }
        self.eat_expected(Token::AngleEnd);
        self.eat_expected(Token::AngleEnd);
        return result;
    }

    #[allow(dead_code)]
    fn print_debug(&self) {
        println!("Debug start...");
        for i in 0..10 {
            println!("{}", self.peek_nth(i));
        }
    }

    fn read_number(&mut self) -> i32 {
        let tok = self.eat_next_token();
        if !is_num(&tok) {
            println!("Expected number, got {}", tok);
        }
        return get_num(&tok);
    }

    fn peek(&self) -> Token {
        return self.tokens[self.offset].1.clone();
    }

    fn peek_nth(&self, n: usize) -> Token {
        return self.tokens[self.offset+n].1.clone();
    }

    fn eat_next_token(&mut self) -> Token {
        let result = self.peek();
        self.offset += 1;
        return result;
    }

    fn eat_expected(&mut self, tok: Token) {
        let result = self.eat_next_token();
        assert_eq!(result, tok);
    }

    fn eat_object(&mut self) -> Block {
        let loc = self.src_loc();
        let num1 = get_num(&self.eat_next_token());
        assert!(is_num(&self.peek()));
        let num2 = get_num(&self.eat_next_token());
        assert!(self.peek() == Token::ObjKeyword);
        self.eat_next_token();

        let value = self.eat_value();
        let result = Block::Object {
            id: num1,
            gxn: num2,
            body: value,
            loc: loc,
        };
        self.expect_end_object();
        return result;
    }

    fn eat_value(&mut self) -> Value {
        if self.peek_is_dict() {
            let result = Value::from_dict(self.eat_dictionary());
            if self.peek() == Token::StreamKeyword {
                return Value::ByteStream(Box::new(result), self.eat_bytestream());
            } else {
                return result;
            }
        } else if self.peek() == Token::AngleStart {
            self.eat_next_token();
            let tok = self.eat_next_token();
            assert!(is_identifier(&tok));
            self.eat_next_token();
            return Value::Identifier(get_id(&tok));
        } else if self.peek() == Token::LeftBracket {
            let mut items = vec![];
            self.eat_next_token();
            while self.peek() != Token::RightBracket {
                assert!(self.peek() != Token::LeftBracket);
                if self.peek() == Token::ForwardSlash {
                    self.eat_next_token();
                    let tok = &self.eat_next_token();
                    assert!(is_identifier(tok));
                    items.push(Value::Identifier(get_id(tok)));
                } else if is_num(&self.peek()) {
                    let v1 = self.read_number();
                    if !is_num(&self.peek()) {
                        items.push(Value::Number(v1));
                        continue;
                    }
                    let v2 = self.read_number();
                    if self.peek() != Token::RefKeyword {
                        items.push(Value::Number(v1));
                        items.push(Value::Number(v2));
                        continue;
                    }
                    assert!(self.eat_next_token() == Token::RefKeyword);
                    items.push(Value::Reference { id: v1, gxn: v2 });
                } else {
                    items.push(self.eat_value());
                }
            }
            self.eat_expected(Token::RightBracket);
            return Value::Vector(Box::new(items));
        } else if is_num(&self.peek()) {
            let vv = self.eat_next_token();
            if is_num(&self.peek()) {
                let v2 = self.eat_next_token();
                assert!(self.eat_next_token() == Token::RefKeyword);
                return Value::Reference { id: get_num(&vv), gxn: get_num(&v2) }
            } else {
                return Value::Number(get_num(&vv));
            }
        } else if self.peek() == Token::LeftParens {
            self.eat_next_token();
            let tok = self.eat_next_token();
            assert!(is_string(&tok));
            assert!(self.eat_next_token() == Token::RightParens);
            return Value::Identifier(get_str(&tok));
        } else if self.peek() == Token::ForwardSlash {
            self.eat_next_token();
            let tok = self.eat_next_token();
            assert!(is_identifier(&tok));
            return Value::Identifier(get_id(&tok));
        } else {
            panic!();
        }
    }
}
