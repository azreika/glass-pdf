use iced::futures::stream;
use crate::content_tokenizer::ContentToken;
use crate::ast::{Font, FontLib};

use crate::viewer_message::{GlyphInfo, Message, State};

struct TextState {
    matrix: Vec<f64>,
    line_matrix: Vec<f64>,
    font: Option<String>,
    size: Option<f64>,
}

#[derive(Debug, Clone)]
enum Value {
    Number(f64),
    Identifier(String),
    Array (Vec<Box<Value>>),
}

impl TextState {
    fn init_matrix() -> Vec<f64> {
        return vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
    }

    fn new() -> Self {
        return TextState {
            matrix: Self::init_matrix(),
            line_matrix: Self::init_matrix(),
            font: None,
            size: None,
        };
    }

    fn matrix_to_3d(v: &Vec<f64>) -> Vec<f64> {
        assert_eq!(v.len(), 6);
        let result = vec![
            v[0], v[1], 0.0,
            v[2], v[3], 0.0,
            v[4], v[5], 1.0,
        ];
        return result;
    }

    fn set_matrices(&mut self, v: &Vec<f64>) {
        let expected_matrix = Self::matrix_to_3d(v);
        self.matrix = expected_matrix.clone();
        self.line_matrix = expected_matrix.clone();
    }
}

pub struct ContentStreamer {
    tokens: Vec<ContentToken>,
    offset: usize,
    text_state: TextState,
    font_lib: FontLib,
    stack: Vec<Value>,
    state: State,
}


impl ContentStreamer {
    fn new(font_lib: FontLib, tokens: Vec<ContentToken>) -> Self {
        return Self {
            tokens,
            offset: 0,
            text_state: TextState::new(),
            font_lib,
            stack: vec![],
            state: State::TopLevel,
        };
    }

    fn reset_text_state(&mut self) {
        self.text_state = TextState::new();
    }

    pub fn stream_content(flib: FontLib, toks: Vec<ContentToken>) -> impl iced::futures::Stream<Item=Message> {
        let p = Self::new(flib, toks);
        return stream::unfold(p, |mut parser| async move {
            iced::futures::future::ready(()).await;
            loop {
                if parser.offset >= parser.tokens.len() {
                    return None;
                }
                let msg = parser.advance();
                // Skip noops to avoid redundant drawings
                if !matches!(msg, Message::Noop) {
                    return Some((msg, parser));
                }
            }
        });
    }

    fn advance(&mut self) -> Message {
        match self.state {
            State::TopLevel => {
                // Keep going until we need to start processing text
                if !matches!(self.peek(), ContentToken::BTKeyword) {
                    self.offset += 1;
                    return Message::Noop;
                }
                assert!(matches!(self.next_token(), ContentToken::BTKeyword));
                assert!(self.stack.is_empty());
                self.state = State::InText;
                return Message::Noop;
            },
            State::InText => {
                match self.peek() {
                    // End Text, go back to Top Level
                    ContentToken::ETKeyword => {
                        self.next_token();
                        assert!(self.stack.is_empty());
                        self.reset_text_state();
                        self.state = State::TopLevel;
                        return Message::Noop;
                    },

                    // Keep processing text
                    _ => {
                        if self.is_operator(&self.peek()) {
                            let tok = self.next_token();
                            let msg = self.process_op(&tok);
                            return msg;
                        }
                        let value = self.parse_value();
                        self.stack.push(value);
                        return Message::Noop;
                    },
                }
            },
        }
    }

    fn next_token(&mut self) -> ContentToken {
        let tok = self.peek();
        self.offset += 1;
        return tok;
    }

    fn is_operator(&self, tok: &ContentToken) -> bool {
        return
            matches!(tok, ContentToken::TmKeyword) ||
            matches!(tok, ContentToken::TfKeyword) ||
            matches!(tok, ContentToken::TjKeyword) ||
            matches!(tok, ContentToken::TJKeyword)
    }

    fn y(&self) -> f64 {
        return self.text_state.matrix[7];
    }

    fn x(&self) -> f64 {
        return self.text_state.matrix[6];
    }

    fn pop_number(stack: &mut Vec<Value>) -> f64 {
        return match stack.pop().unwrap() {
            Value::Number(v) => v,
            other => panic!("expected number, got {:?}", other)
        };
    }

    fn pop_string(stack: &mut Vec<Value>) -> String {
        return match stack.pop().unwrap() {
            Value::Identifier(v) => v,
            other => panic!("expected number, got {:?}", other)
        };
    }

    fn pop_array(stack: &mut Vec<Value>) -> Vec<Box<Value>> {
        return match stack.pop().unwrap() {
            Value::Array(arr) => arr,
            other => panic!("expected number, got {:?}", other)
        };
    }

    fn curr_size(&self) -> f32 {
        let mm = self.text_state.size;
        return match mm {
            Some(vv) => vv as f32,
            _ => 16.0,
        };
    }

    fn get_font(&self) -> &Font {
        return match self.text_state.font {
            None => panic!(),
            Some(ref other) => self.font_lib.get_font(other.to_string()),
        }
    }

    fn char_width(&self, c: char) -> f64 {
        let font = self.get_font();
        let width = font.get_width(c) as f64;
        let size = self.text_state.size.unwrap();
        return (width * size) / 1000.0;
    }

    fn mk_message(&mut self, str: &str) -> Message {
        let x_scale = self.text_state.matrix[0] as f32;
        let init_size = self.curr_size();
        let size = init_size * x_scale;
        let x_pos = self.x();
        let y_pos = self.y();

        let mut messages = vec![];
        let mut curr_x = x_pos;
        for s in str.chars().into_iter() {
            messages.push(Message::DrawGlyph(GlyphInfo{
                x: curr_x as i32,
                y: y_pos as i32,
                str: s.to_string(),
                size: size,
            }));
            curr_x += self.char_width(s) as f64 * x_scale as f64;
        }
        self.set_x(curr_x);
        return self.mk_message_block(messages);
    }

    fn mk_message_block(&mut self, msgs: Vec<Message>) -> Message {
        let msgs = msgs.into_iter().filter(|c| !matches!(c, Message::Noop)).collect();
        return Message::DrawBlock(msgs);
    }

    fn process_op(&mut self, tok: &ContentToken) -> Message {
        let stack = &mut self.stack;
        match tok {
            ContentToken::TmKeyword => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(Self::pop_number(stack));
                }
                mat.reverse();
                self.text_state.set_matrices(&mat);
                return Message::Noop;
            },
            ContentToken::TfKeyword => {
                let size = Self::pop_number(stack);
                let font = Self::pop_string(stack);
                self.text_state.font = Some(font);
                self.text_state.size = Some(size);
                return Message::Noop;
            },
            ContentToken::TjKeyword => {
                // show one
                let str = Self::pop_string(stack);
                return self.mk_message(&str);
            },
            ContentToken::TJKeyword => {
                // show one or mroe
                let arr = Self::pop_array(stack);
                let mut msgs = vec![];
                for val in arr {
                    let msg = match *val {
                        Value::Number(x) => {
                            self.move_x(x);
                            Message::Noop
                        }
                        Value::Identifier(id) => self.mk_message(&id.to_string()),
                        other => panic!("unexpected array value {:?}", other),
                    };
                    msgs.push(msg);
                }
                return self.mk_message_block(msgs);
            }
            _ => panic!("Unexpected operator {:?}", tok),
        }
    }

    fn set_x(&mut self, x: f64) {
        self.text_state.matrix[6] = x;
    }
    fn move_x(&mut self, x: f64) {
        self.text_state.matrix[6] -= (x * self.text_state.size.unwrap())/1000.0;
    }

    fn parse_value(&mut self) -> Value {
        let tok = self.next_token();
        return match tok {
            ContentToken::Identifier(id) => Value::Identifier(id.to_string()),
            ContentToken::Number(x) => Value::Number(x),
            ContentToken::LParens => self.parse_parens(),
            ContentToken::LBracket => self.parse_array(),
            _ => panic!("Unexpected token {:?}", tok),
        }
    }

    fn peek(&self) -> ContentToken {
        return self.tokens[self.offset].clone();
    }

    fn parse_parens(&mut self) -> Value {
        let result = self.parse_value();
        assert!(matches!(self.next_token(), ContentToken::RParens));
        return result;
    }

    fn parse_array(&mut self) -> Value {
        let mut arr = vec![];

        while !matches!(self.peek(), ContentToken::RBracket) {
            assert!(!matches!(self.peek(), ContentToken::LBracket));
            let expr = self.parse_value();
            arr.push(Box::new(expr));
        }
        self.next_token();
        return Value::Array(arr);
    }
}
