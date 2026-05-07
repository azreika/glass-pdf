use encoding_rs::MACINTOSH;
use iced::futures::stream;
use crate::content_tokenizer::ContentToken;
use crate::fonts::{Font, FontLib};

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

#[derive(Clone,Debug)]
struct GraphicsState {
    ctm: Vec<f64>,
}

fn multiply_3d(v1: &Vec<f64>, v2: &Vec<f64>) -> Vec<f64> {
    let mut result = vec![0.0; 9];
    assert_eq!(v1.len(), 9);
    assert_eq!(v2.len(), 9);

    for row in 0..3 {
        for col in 0..3 {
            for k in 0..3 {
                result[row*3 + col] += v1[row*3 + k] * v2[k*3 + col];
            }
        }
    }
    return result;
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

impl GraphicsState {
    fn init_matrix() -> Vec<f64> {
        return vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0
        ];
    }

    fn new() -> Self {
        return GraphicsState {
            ctm: Self::init_matrix(),
        };
    }

}

impl TextState {
    fn init_matrix() -> Vec<f64> {
        return vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0];
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
    graphics_state_stack: Vec<GraphicsState>,
    graphics_state: GraphicsState,
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
            graphics_state_stack: vec![],
            graphics_state: GraphicsState::new(),
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
                let tok = &self.peek();

                if matches!(tok, ContentToken::BTKeyword) {
                    self.next_token();
                    self.state = State::InText;
                    return Message::Noop;
                }

                if self.is_main_operator(tok) {
                    let tok = self.next_token();
                    let msg = self.process_main_op(&tok);
                    return msg;
                }

                let value = match tok {
                    ContentToken::Number(x) => Value::Number(*x),
                    ContentToken::Identifier(id) => Value::Identifier(id.to_string()),
                    _ => {
                        println!("STACK: {:?}", self.stack);
                        panic!("Unhandled TopLevel: {:?}", self.peek())
                    },
                };
                self.stack.push(value);
                self.offset += 1;
                return Message::Noop;
            },
            State::InText => {
                match self.peek() {
                    // End Text, go back to Top Level
                    ContentToken::ETKeyword => {
                        self.next_token();
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
            matches!(tok, ContentToken::TJKeyword);
    }

    fn is_main_operator(&self, tok: &ContentToken) -> bool {
        return
            matches!(tok, ContentToken::SaveGraphicsState) ||
            matches!(tok, ContentToken::RestoreGraphicsState) ||
            matches!(tok, ContentToken::RectKeyword) ||
            matches!(tok, ContentToken::WKeyword) ||
            matches!(tok, ContentToken::NKeyword) ||
            matches!(tok, ContentToken::CsStroke) ||
            matches!(tok, ContentToken::EndCsStroke) ||
            matches!(tok, ContentToken::Fill) ||
            matches!(tok, ContentToken::IKeyword) ||
            matches!(tok, ContentToken::CmStroke)
            ;
    }

    fn process_main_op(&mut self, tok: &ContentToken) -> Message {
        match tok {
            ContentToken::SaveGraphicsState => {
                self.graphics_state_stack.push(self.graphics_state.clone());
                return Message::Noop;
            },
            ContentToken::RestoreGraphicsState => {
                self.graphics_state = self.graphics_state_stack.pop().unwrap();
                return Message::Noop;
            },
            ContentToken::RectKeyword => {
                let height = self.pop_number();
                let width = self.pop_number();
                let y = self.pop_number();
                let x = self.pop_number();
                println!("TODO: implement rectangle thing");
                return Message::Noop;
            },
            ContentToken::WKeyword => {
                // Clipping Path Operator
                println!("TODO: implement clipping path operator W");
                return Message::Noop;
            },
            ContentToken::NKeyword => {
                // Clipping Path Operator - end path object without filling it
                println!("TODO: implement clipping path operator N");
                return Message::Noop;
            },
            ContentToken::CsStroke => {
                let cs = self.pop_string();
                println!("TODO: implement colour space operator cs");
                return Message::Noop;
            },
            ContentToken::EndCsStroke => {
                let sc = self.pop_number();
                println!("TODO: implement colour space operator sc");
                return Message::Noop;
            },
            ContentToken::Fill => {
                println!("TODO: implement colour space operator fill");
                return Message::Noop;
            },
            ContentToken::IKeyword => {
                println!("TODO: implement colour space operator flatness I");
                let flatness = self.pop_number();
                return Message::Noop;
            },
            ContentToken::CmStroke => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(self.pop_number());
                }
                mat.reverse();

                let mat = matrix_to_3d(&mat);
                let result = multiply_3d(self.curr_ctm(), &mat);
                self.graphics_state.ctm = result;
                return Message::Noop;
            }
            _ => panic!(),
        }
    }

    fn curr_ctm(&self) -> &Vec<f64> {
        return &self.graphics_state.ctm;
    }

    fn text_x(&self) -> f64 {
        return self.text_state.matrix[6];
    }

    fn pop_number(&mut self) -> f64 {
        return match self.stack.pop().unwrap() {
            Value::Number(v) => v,
            other => panic!("expected number, got {:?}", other)
        };
    }

    fn pop_string(&mut self) -> String {
        return match self.stack.pop().unwrap() {
            Value::Identifier(v) => v,
            other => panic!("expected number, got {:?}", other)
        };
    }

    fn pop_array(&mut self) -> Vec<Box<Value>> {
        return match self.stack.pop().unwrap() {
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

    fn char_width(&self, c: u8) -> f64 {
        let font = self.get_font();
        let width = font.get_width(c) as f64;
        let size = self.text_state.size.unwrap();
        return (width * size) / 1000.0;
    }

    fn mk_message(&mut self, str: &str) -> Message {
        let text_x_scale = self.text_state.matrix[0] as f32;

        let mut messages = vec![];

        let bytes = str.as_bytes();
        let (result, real_encoding, any_malformed) = MACINTOSH.decode(bytes);
        println!("BYTES: {:?}", bytes);
        assert_eq!(real_encoding, MACINTOSH);
        assert!(!any_malformed);
        let decoded = result.into_owned();
        let chars: Vec<char> = decoded.chars().collect();

        assert_eq!(chars.len(), bytes.len());

        for (byte, unicode_char) in bytes.iter().zip(chars.iter()) {
            let ctm = self.curr_ctm();
            let effective = multiply_3d(&self.text_state.matrix, ctm);
            let screen_x = effective[6];
            let screen_y = effective[7];

            let size = self.curr_size() * effective[4].abs() as f32;
            messages.push(Message::DrawGlyph(GlyphInfo{
                x: screen_x as i32,
                y: screen_y as i32,
                str: unicode_char.to_string(),
                size: size,
                font: self.get_font().clone(),
            }));
            println!("{:?}", unicode_char.to_string());
            let new_x = self.text_x() + self.char_width(*byte) as f64 * text_x_scale as f64;
            self.set_x(new_x);
        }
        return self.mk_message_block(messages);
    }

    fn mk_message_block(&mut self, msgs: Vec<Message>) -> Message {
        let msgs = msgs.into_iter().filter(|c| !matches!(c, Message::Noop)).collect();
        return Message::DrawBlock(msgs);
    }

    fn process_op(&mut self, tok: &ContentToken) -> Message {
        match tok {
            ContentToken::TmKeyword => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(self.pop_number());
                }
                mat.reverse();
                self.text_state.set_matrices(&mat);
                return Message::Noop;
            },
            ContentToken::TfKeyword => {
                let size = self.pop_number();
                let font = self.pop_string();
                self.text_state.font = Some(font);
                self.text_state.size = Some(size);
                return Message::Noop;
            },
            ContentToken::TjKeyword => {
                // show one
                let str = self.pop_string();
                return self.mk_message(&str);
            },
            ContentToken::TJKeyword => {
                // show one or mroe
                let arr = self.pop_array();
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
            },
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
