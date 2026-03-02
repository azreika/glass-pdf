use crate::tokenizer::{Token,SrcLoc};
use crate::ast::{Pdf,Block,Value};

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
            let loc = self.src_loc();
            match tok {
                Token::Percent => {
                    self.eat_next_token();
                    assert!(is_string(&self.peek()));
                    self.eat_next_token();
                },
                Token::Number(v1) => {
                    self.eat_next_token();
                    assert!(is_num(&self.peek()));
                    let num2 = get_num(&self.eat_next_token());
                    assert!(self.peek() == Token::ObjKeyword);
                    self.eat_next_token();

                    if self.peek() == Token::AngleStart {
                        self.eat_dictionary();
                        if self.peek() == Token::StreamKeyword {
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
                            assert!(self.eat_next_token() == Token::EndObjKeyword);
                            let stream = Value::ByteStream(bytes);

                            blocks.push(Block::Object {
                                id: v1,
                                gxn: num2,
                                body: stream,
                                loc: loc,
                            });
                        } else {
                            self.eat_expected(Token::EndObjKeyword);
                        }
                    } else if self.peek() == Token::LeftBracket {
                        self.eat_next_token();
                        while self.peek() != Token::RightBracket {
                            if self.peek() == Token::ForwardSlash {
                                self.eat_next_token();
                                assert!(is_identifier(&self.eat_next_token()));
                            } else if self.peek() == Token::RefKeyword {
                                self.eat_next_token();
                            } else {
                                assert!(is_num(&self.eat_next_token()));
                            }
                        }
                        self.eat_expected(Token::RightBracket);
                        self.eat_expected(Token::EndObjKeyword);
                    } else {
                        assert!(is_num(&self.peek()));
                        let v = get_num(&self.eat_next_token());
                        blocks.push(Block::Object {
                            id: v1,
                            gxn: num2,
                            body: Value::Number(v),
                            loc: loc,
                        });
                        self.eat_expected(Token::EndObjKeyword);
                    }
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
                    self.eat_dictionary();
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

    fn src_loc(&self) -> SrcLoc {
        return self.tokens[self.offset].0;
    }

    fn eat_dictionary(&mut self) {
        self.eat_expected(Token::AngleStart);

        if self.peek() != Token::AngleStart {
            assert!(is_identifier(&self.eat_next_token()));
            self.eat_expected(Token::AngleEnd);
            return;
        }

        self.eat_expected(Token::AngleStart);
        loop {
            match self.peek() {
                Token::AngleStart => {
                    self.eat_dictionary();
                },
                Token::AngleEnd => {
                    break;
                },
                _ => {
                    self.eat_next_token();
                }
            }
        }
        self.eat_expected(Token::AngleEnd);
        self.eat_expected(Token::AngleEnd);
    }

    fn peek(&self) -> Token {
        return self.tokens[self.offset].1.clone();
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
}
