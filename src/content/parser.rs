use crate::content::tokenizer::Token;

struct Parser {
    toks: Vec<Token>,
    offset: usize,
}

#[derive(Clone, Debug, PartialEq)]
struct ContentAst {
    ops: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Op(OpName),
    Array(Vec<Value>),
    Number(f64),
    StringBytes(Vec<u8>),
}

#[derive(Clone, Debug, PartialEq, Copy)]
enum OpName {
    BeginText,
    TJ,
}

impl Parser {
    fn new(toks: Vec<Token>) -> Self {
        return Parser {
            toks,
            offset: 0,
        };
    }

    fn peek(&self) -> &Token {
        return &self.toks[self.offset];
    }

    fn lex_token(&mut self) -> Token {
        let tok = self.peek().clone();
        self.offset += 1;
        return tok;
    }

    fn lex_array(&mut self) -> Value {
        let mut arr = vec![];
        assert!(matches!(self.lex_token(), Token::LBracket));

        while !matches!(self.peek(), Token::RBracket) {
            let tok = self.lex_token();
            match tok {
                Token::Number(num) => arr.push(Value::Number(num)),
                _ => panic!("Expected number, got {:?}", tok),
            }
        }

        assert!(matches!(self.lex_token(), Token::RBracket));
        return Value::Array(arr);
    }

    fn run(&mut self) -> ContentAst {
        self.offset = 0;
        let mut ops = vec![];

        while self.offset < self.toks.len() {
            match self.peek() {
                Token::LBracket => { ops.push(self.lex_array()); },
                _ => panic!(),
            }
        }

        return ContentAst {
            ops,
        };
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     fn parse(toks: Vec<Token>) -> ContentAst {
//         let mut parser = Parser::new(toks);
//         return parser.run();
//     }

//     #[test]
//     fn simple_array() {
//         let toks = vec![
//             Token::LBracket,
//             Token::Number(0.0),
//             Token::Number(1.0),
//             Token::RBracket,
//         ];

//         let actual = parse(toks);
//         let expected = ContentAst {
//             ops: vec![
//                 Value::Number(0.0)
//                 Op::TJ(Value::Array(vec![
//                     Value::Number(0.0),
//                     Value::Number(1.0)
//                 ])),
//             ],
//         };
//         assert_eq!(actual, expected);
//     }

//     #[test]
//     fn simple_text() {
//         let toks = vec![
//             Token::BTKeyword,

//             Token::Identifier("F1".to_string()),
//             Token::Number(16.0),
//             Token::TfKeyword,

//             Token::StringBytes("hello".to_string().as_bytes().to_vec()),
//             Token::TjKeyword,

//             Token::ETKeyword,
//         ];

//         let actual = parse(toks);
//         let expected = ContentAst {
//             ops: vec![
//                 Op::BeginText,
//                 Op::Tf(
//                     Value::Identifier("F1".to_string()),
//                     Value::Number(16.0)),


//             ],
//         };
//         assert_eq!(actual, expected);
//     }
// }
