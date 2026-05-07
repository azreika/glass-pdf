use crate::pdf_tokenizer::PdfToken;
use crate::src_loc::SrcLoc;
use crate::ast::{Pdf,Block,Value};
use std::collections::HashMap;

struct Parser<'a> {
    tokens: &'a Vec<(SrcLoc, PdfToken)>,
    offset: usize,
}

fn is_string(tok: &PdfToken) -> bool {
    return match tok {
        PdfToken::String(_) => true,
        _ => false,
    }
}

fn is_identifier(tok: &PdfToken) -> bool {
    return match tok {
        PdfToken::Identifier(_) => true,
        _ => false,
    }
}

fn is_num(tok: &PdfToken) -> bool {
    return match tok {
        PdfToken::Number(_) => true,
        _ => false,
    }
}

fn is_bytestream(tok: &PdfToken) -> bool {
    return match tok {
        PdfToken::ByteStream(_) => true,
        _ => false,
    }
}

fn get_id(tok: &PdfToken) -> String {
    return match tok {
        PdfToken::Identifier(x) => x.to_string(),
        _ => {
            assert!(false);
            "".to_string()
        }
    }
}

fn get_str(tok: &PdfToken) -> String {
    return match tok {
        PdfToken::String(x) => x.to_string(),
        _ => {
            assert!(false);
            "".to_string()
        }
    }
}


fn get_num(tok: &PdfToken) -> f32 {
    return match tok {
        PdfToken::Number(v) => *v,
        _ => panic!(),
    };
}

fn get_bytes(tok: &PdfToken) -> Vec<u8> {
    return match tok {
        PdfToken::ByteStream(bytes) => bytes.clone().to_vec(),
        _ => {
            assert!(false);
            vec![]
        }
    }
}

pub fn parse_tokens(tokens: &Vec<(SrcLoc, PdfToken)>) -> Pdf {
    let mut parser = Parser { tokens, offset: 0 };
    return parser.run_parser();
}

impl Parser<'_> {
    fn run_parser(&mut self) -> Pdf {
        let mut blocks = vec![];
        let mut start_xref = 0.0;
        let mut xref = vec![];
        while self.offset < self.tokens.len() {
            let tok = self.peek();
            match tok {
                PdfToken::Percent => {
                    self.eat_next_token();
                    assert!(is_string(&self.peek()));
                    self.eat_next_token();
                },
                PdfToken::Number(_) => {
                    let obj = self.eat_object();
                    blocks.push(obj);
                },
                PdfToken::XRefKeyword => {
                    self.eat_next_token();
                    while self.peek() != PdfToken::TrailerKeyword {
                        let v1 = self.eat_next_token();
                        assert!(is_num(&v1));
                        let v2 = self.eat_next_token();
                        assert!(is_num(&v2));

                        // TODO: fix up IDs here
                        let start_id = get_num(&v1);
                        let num_objs = get_num(&v2);

                        for _ in 0..num_objs as usize {
                            let v1 = self.eat_next_token();
                            assert!(is_num(&v1));
                            let offset = get_num(&v1);
                            let v2 = self.eat_next_token();
                            assert!(is_num(&v2));
                            let kk = self.eat_next_token();
                            println!("hii? {:?} {:?} {:?}", kk, v1, v2);
                            assert!(kk == PdfToken::FKeyword || kk == PdfToken::NKeyword);
                            xref.push(SrcLoc::new(offset as usize));
                        }
                    }
                },
                PdfToken::TrailerKeyword => {
                    self.eat_next_token();
                    let result = self.eat_dictionary();
                    blocks.push(Block::Trailer(result));
                    self.eat_expected(PdfToken::StartXRefKeyword);
                    let v1 = self.eat_next_token();
                    assert!(is_num(&v1));
                    // TODO: what if we have multiple xrefs?
                    start_xref = get_num(&v1);
                    self.eat_expected(PdfToken::EOFKeyword);
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
        if self.peek() != PdfToken::AngleStart {
            return false;
        }
        return self.peek_nth(1) == PdfToken::AngleStart;
    }

    fn eat_bytestream(&mut self) -> Vec<u8> {
        assert!(self.eat_next_token() == PdfToken::StreamKeyword);
        assert!(is_bytestream(&self.peek()));
        let mut bytes = vec![];
        let mut bytes_tok = self.eat_next_token();
        while is_bytestream(&bytes_tok) {
            for bb in get_bytes(&bytes_tok) {
                bytes.push(bb);
            }
            bytes_tok = self.eat_next_token();
        }
        assert!(bytes_tok == PdfToken::EndStreamKeyword);
        return bytes;
    }

    fn src_loc(&self) -> SrcLoc {
        return self.tokens[self.offset].0;
    }

    fn expect_end_object(&mut self) {
        let token = self.eat_next_token();
        if token != PdfToken::EndObjKeyword {
            println!("Expected end of object, but got `{}`.", token);
            self.throw_error();
        }
    }

    fn throw_error(&self) {
        panic!("failed");
    }

    fn eat_dictionary(&mut self) -> HashMap<String,Value> {
        let mut result = HashMap::new();

        self.eat_expected(PdfToken::AngleStart);
        self.eat_expected(PdfToken::AngleStart);

        loop {
            if self.peek() == PdfToken::AngleEnd {
                break;
            }
            assert!(self.eat_next_token() == PdfToken::ForwardSlash);
            let tok = self.eat_next_token();
            assert!(is_identifier(&tok));

            let val = self.eat_value();
            result.insert(get_id(&tok), val);
        }
        self.eat_expected(PdfToken::AngleEnd);
        self.eat_expected(PdfToken::AngleEnd);
        return result;
    }

    #[allow(dead_code)]
    fn print_debug(&self) {
        println!("Debug start...");
        for i in 0..10 {
            println!("{}", self.peek_nth(i));
        }
    }

    fn read_number(&mut self) -> f32 {
        let tok = self.eat_next_token();
        if !is_num(&tok) {
            println!("Expected number, got {}", tok);
        }
        return get_num(&tok);
    }

    fn peek(&self) -> PdfToken {
        return self.tokens[self.offset].1.clone();
    }

    fn peek_nth(&self, n: usize) -> PdfToken {
        return self.tokens[self.offset+n].1.clone();
    }

    fn eat_next_token(&mut self) -> PdfToken {
        let result = self.peek();
        self.offset += 1;
        return result;
    }

    fn eat_expected(&mut self, tok: PdfToken) {
        let result = self.eat_next_token();
        assert_eq!(result, tok);
    }

    fn eat_object(&mut self) -> Block {
        let loc = self.src_loc();
        let num1 = get_num(&self.eat_next_token());
        assert!(is_num(&self.peek()));
        let num2 = get_num(&self.eat_next_token());
        assert!(self.peek() == PdfToken::ObjKeyword);
        self.eat_next_token();

        let value = self.eat_value();
        let result = Block::Object {
            id: num1 as i32,
            gxn: num2 as i32,
            body: value,
            loc: loc,
        };
        self.expect_end_object();
        return result;
    }

    fn eat_value(&mut self) -> Value {
        if self.peek_is_dict() {
            let result = Value::from_dict(self.eat_dictionary());
            if self.peek() == PdfToken::StreamKeyword {
                return Value::ByteStream(Box::new(result), self.eat_bytestream());
            } else {
                return result;
            }
        } else if self.peek() == PdfToken::AngleStart {
            self.eat_next_token();
            let tok = self.eat_next_token();
            assert!(is_identifier(&tok));
            self.eat_next_token();
            return Value::Identifier(get_id(&tok));
        } else if self.peek() == PdfToken::LeftBracket {
            let mut items = vec![];
            self.eat_next_token();
            while self.peek() != PdfToken::RightBracket {
                if self.peek() == PdfToken::ForwardSlash {
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
                    if self.peek() != PdfToken::RefKeyword {
                        items.push(Value::Number(v1));
                        items.push(Value::Number(v2));
                        continue;
                    }
                    assert!(self.eat_next_token() == PdfToken::RefKeyword);
                    items.push(Value::Reference { id: v1 as i32, gxn: v2 as i32});
                } else if matches!(self.peek(), PdfToken::LeftBracket) {
                    items.push(self.eat_value());
                } else {
                    items.push(self.eat_value());
                }
            }
            self.eat_expected(PdfToken::RightBracket);
            return Value::Vector(items);
        } else if is_num(&self.peek()) {
            let vv = self.eat_next_token();
            if is_num(&self.peek()) {
                let v2 = self.eat_next_token();
                assert!(self.eat_next_token() == PdfToken::RefKeyword);
                return Value::Reference { id: get_num(&vv) as i32, gxn: get_num(&v2) as i32 }
            } else {
                return Value::Number(get_num(&vv));
            }
        } else if self.peek() == PdfToken::LeftParens {
            self.eat_next_token();
            let tok = self.eat_next_token();
            assert!(is_string(&tok));
            assert!(self.eat_next_token() == PdfToken::RightParens);
            return Value::Identifier(get_str(&tok));
        } else if self.peek() == PdfToken::ForwardSlash {
            self.eat_next_token();
            let tok = self.eat_next_token();
            assert!(is_identifier(&tok));
            return Value::Identifier(get_id(&tok));
        } else if self.peek() == PdfToken::BooleanTrue {
            self.eat_next_token();
            return Value::Boolean(true);
        } else if self.peek() == PdfToken::BooleanFalse {
            self.eat_next_token();
            return Value::Boolean(false);
        } else if self.peek() == PdfToken::Null {
            self.eat_next_token();
            return Value::Null;
        } else {
            println!("Unexpected value: {:?}", self.peek());
            panic!();
        }
    }
}
