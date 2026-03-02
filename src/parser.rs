use crate::tokenizer::{Token};
use crate::ast::{Pdf,Object,ByteStream};

struct Parser<'a> {
    tokens: &'a Vec<Token>,
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

pub fn parse_tokens(tokens: &Vec<Token>) -> Pdf {
    let mut parser = Parser { tokens, offset: 0 };
    return parser.run_parser();
}

impl Parser<'_> {
    fn run_parser(&mut self) -> Pdf {
        let mut objects = vec![];
        while self.offset < self.tokens.len() {
            let tok = &self.tokens[self.offset];
            match tok {
                Token::Percent => {
                    self.eat_next_token();
                    assert!(is_string(self.peek()));
                    self.eat_next_token();
                },
                Token::Number(v1) => {
                    self.eat_next_token();
                    assert!(is_num(self.peek()));
                    let num2 = get_num(self.eat_next_token());
                    assert!(*self.peek() == Token::ObjKeyword);
                    self.eat_next_token();

                    if *self.peek() == Token::AngleStart {
                        self.eat_dictionary();
                        if *self.peek() == Token::StreamKeyword {
                            assert!(*self.eat_next_token() == Token::StreamKeyword);
                            assert!(is_bytestream(self.peek()));
                            let mut bytes = vec![];
                            let mut bytes_tok = self.eat_next_token();
                            while is_bytestream(bytes_tok) {
                                for bb in get_bytes(&bytes_tok) {
                                    bytes.push(bb);
                                }
                                bytes_tok = self.eat_next_token();
                            }
                            assert!(*bytes_tok == Token::EndStreamKeyword);
                            assert!(*self.eat_next_token() == Token::EndObjKeyword);
                            objects.push(Object::Stream {
                                id: *v1,
                                generation: num2,
                                body: ByteStream {bytes},
                            });
                        } else {
                            println!("{:?} {:?}", self.peek(), self.peek_nth(1));
                            assert!(*self.eat_next_token() == Token::EndObjKeyword);
                        }
                    } else if *self.peek() == Token::LeftBracket {
                        self.eat_next_token();
                        while *self.peek() != Token::RightBracket {
                            if *self.peek() == Token::ForwardSlash {
                                self.eat_next_token();
                                assert!(is_identifier(self.eat_next_token()));
                            } else if *self.peek() == Token::RefKeyword {
                                self.eat_next_token();
                            } else {
                                println!("{:?}", self.peek());
                                assert!(is_num(self.eat_next_token()));
                            }
                        }
                        assert!(*self.eat_next_token() == Token::RightBracket);
                        assert!(*self.eat_next_token() == Token::EndObjKeyword);
                    } else {
                        assert!(is_num(self.peek()));
                        let v = get_num(self.eat_next_token());
                        objects.push(Object::Number(v));
                        assert!(*self.eat_next_token() == Token::EndObjKeyword);
                    }
                },
                Token::XRefKeyword => {
                    self.eat_next_token();
                    let v1 = self.eat_next_token();
                    assert!(is_num(v1));
                    let v2 = self.eat_next_token();
                    assert!(is_num(v2));

                    while *self.peek() != Token::TrailerKeyword {
                        let v1 = self.eat_next_token();
                        assert!(is_num(v1));
                        let v2 = self.eat_next_token();
                        assert!(is_num(v2));
                        let kk = self.eat_next_token();
                        assert!(*kk == Token::FKeyword || *kk == Token::NKeyword);
                    }
                },
                Token::TrailerKeyword => {
                    self.eat_next_token();
                    self.eat_dictionary();
                    assert!(*self.eat_next_token() == Token::StartXRefKeyword);
                    let v1 = self.eat_next_token();
                    assert!(is_num(v1));
                    assert!(*self.eat_next_token() == Token::EOFKeyword);
                },
                _ => {
                    println!("{:?}", objects);
                    println!("{:?}", tok);
                    assert!(false);
                },
            }

        }

        return Pdf { objects };
    }

    fn eat_dictionary(&mut self) {
        assert!(*self.eat_next_token() == Token::AngleStart);

        if *self.peek() != Token::AngleStart {
            println!("{:?}", self.peek());
            assert!(is_identifier(self.eat_next_token()));
            println!("{:?}", self.peek());
            assert!(*self.eat_next_token() == Token::AngleEnd);
            return;
        }

        assert!(*self.eat_next_token() == Token::AngleStart);
        loop {
            let tok = self.peek();
            if *tok == Token::AngleStart {
                self.eat_dictionary();
            } else if *tok == Token::AngleEnd {
                break;
            } else {
                self.eat_next_token();
            }
        }
        assert!(*self.eat_next_token() == Token::AngleEnd);
        assert!(*self.eat_next_token() == Token::AngleEnd);
    }

    fn peek(&self) -> &Token {
        return &self.tokens[self.offset];
    }

    fn peek_nth(&self, n: usize) -> &Token {
        return &self.tokens[self.offset + n];
    }

    fn eat_next_token(&mut self) -> &Token {
        let result = &self.tokens[self.offset];
        self.offset += 1;
        return result;
    }
}
