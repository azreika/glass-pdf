use std::collections::HashMap;

use iced::futures::stream;
use crate::content::tokenizer::Token;
use crate::fonts::Font;

use crate::viewer::PageCtx;
use crate::viewer_message::{GlyphInfo, Message, State};
use crate::transform::{Matrix, multiply_3d};

struct TextState {
    matrix: Matrix,
    font: Option<String>,
    size: Option<f64>,
}

#[derive(Debug, Clone)]
enum Value {
    Number(f64),
    Identifier(String),
    Array(Vec<Box<Value>>),
    StringBytes(Vec<u8>),
    Dict(HashMap<String,Value>),
}

#[derive(Clone,Debug)]
struct GraphicsState {
    ctm: Matrix,
    cs_nostroke: Option<String>,
    colour_nostroke: Option<Vec<f64>>,
}

impl GraphicsState {
    fn new() -> Self {
        return GraphicsState {
            ctm: Matrix::new(),
            cs_nostroke: None,
            colour_nostroke: None,
        };
    }
}

impl TextState {
    fn new() -> Self {
        return TextState {
            matrix: Matrix::new(),
            font: None,
            size: None,
        };
    }
}

pub struct ContentStreamer {
    state: State,
    tokens: Vec<Token>,
    stack: Vec<Value>,
    offset: usize,

    text_state: TextState,

    graphics_state: GraphicsState,
    graphics_state_stack: Vec<GraphicsState>,

    ctx: PageCtx,
}

pub fn stream_content(p: ContentStreamer) -> impl iced::futures::Stream<Item=Message> {
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

impl ContentStreamer {
    pub fn new(ctx: PageCtx, tokens: Vec<Token>) -> Self {
        return Self {
            tokens,
            offset: 0,
            text_state: TextState::new(),
            ctx,
            stack: vec![],
            state: State::TopLevel,
            graphics_state_stack: vec![],
            graphics_state: GraphicsState::new(),
        };
    }

    fn reset_text_state(&mut self) {
        self.text_state = TextState::new();
    }


    fn advance(&mut self) -> Message {
        match self.state {
            State::TopLevel => {
                let tok = &self.peek();

                if self.try_process_op(tok) {
                    self.next_token();
                } else {
                    let value = self.parse_value();
                    self.stack.push(value);
                }
                return Message::Noop;
            },
            State::InText => {
                match self.peek() {
                    // End Text, go back to Top Level
                    Token::ETKeyword => {
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

    fn next_token(&mut self) -> Token {
        let tok = self.peek();
        self.offset += 1;
        return tok;
    }

    fn is_operator(&self, tok: &Token) -> bool {
        return
            matches!(tok, Token::TmKeyword) ||
            matches!(tok, Token::TfKeyword) ||
            matches!(tok, Token::TjKeyword) ||
            matches!(tok, Token::TJKeyword) ||
            matches!(tok, Token::GSKeyword);
    }

    fn num_colour_components(&self) -> u8 {
        let cs = self.graphics_state.cs_nostroke.as_ref().unwrap();
        return self.ctx.cs_lib.num_components(cs.to_string());
    }

    fn set_colour_nostroke(&mut self, vv: Vec<f64>) {
        self.graphics_state.colour_nostroke = Some(vv);
    }

    fn try_process_op(&mut self, tok: &Token) -> bool {
        match tok {
            Token::BTKeyword => {
                self.state = State::InText;
                return true;
            },
            Token::SaveGraphicsState => {
                self.graphics_state_stack.push(self.graphics_state.clone());
                return true;
            },
            Token::RestoreGraphicsState => {
                self.graphics_state = self.graphics_state_stack.pop().unwrap();
                return true;
            },
            Token::RectKeyword => {
                let _height = self.pop_number();
                let _width = self.pop_number();
                let _y = self.pop_number();
                let _x = self.pop_number();
                println!("TODO: implement rectangle thing");
                return true;
            },
            Token::WKeyword | Token::WStarKeyword => {
                // Clipping Path Operator
                println!("TODO: implement clipping path operator W/W*");
                return true;
            },
            Token::NKeyword => {
                // Clipping Path Operator - end path object without filling it
                println!("TODO: implement clipping path operator N");
                return true;
            },
            Token::CsNoStroke => {
                let cs = self.pop_string();
                self.set_cs_nostroke(cs);
                return true;
            },
            Token::GSKeyword => {
                let _cs = self.pop_string();
                println!("TODO: implement gs keyword");
                return true;
            },
            Token::SetColourNoStroke => {
                let num_components = self.num_colour_components();
                let mut vv = vec![];
                for _ in 0..num_components {
                    vv.push(self.pop_number());
                }
                vv.reverse();
                self.set_colour_nostroke(vv);
                return true;
            },
            Token::Fill => {
                println!("TODO: implement colour space operator fill");
                return true;
            },
            Token::IKeyword => {
                println!("TODO: implement colour space operator flatness I");
                let _flatness = self.pop_number();
                return true;
            },
            Token::CmStroke => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(self.pop_number());
                }
                mat.reverse();
                let mat = Matrix::vec6_to_matrix(&mat);
                let result = multiply_3d(self.curr_ctm(), &mat);
                self.graphics_state.ctm = result;
                return true;
            },
            Token::MKeyword | Token::LKeyword => {
                let _y = self.pop_number();
                let _x = self.pop_number();
                println!("TODO: implement M and L keyword");
                return true;
            },
            Token::VKeyword | Token::YKeyword => {
                let _x1 = self.pop_number();
                let _x2 = self.pop_number();
                let _x3 = self.pop_number();
                let _x4 = self.pop_number();
                println!("TODO: implement V and Y keyword");
                return true;
            },
            Token::HKeyword => {
                println!("TODO: implement H keyword");
                return true;
            },
            Token::BMCKeyword => {
                let _dict = self.pop_dict();
                println!("TODO: implement BMC keyword");
                return true;
            }
            Token::EMCKeyword => {
                println!("TODO: implement EMC keyword");
                return true;
            }
            _ => return false,
        }
    }

    fn curr_ctm(&self) -> &Matrix {
        return &self.graphics_state.ctm;
    }

    fn text_x(&self) -> f64 {
        return self.text_state.matrix.x();
    }

    fn pop_number(&mut self) -> f64 {
        return match self.stack.pop().unwrap() {
            Value::Number(v) => v,
            other => panic!("expected number, got {:?}", other)
        };
    }

    fn pop_dict(&mut self) -> HashMap<String,Value> {
        return match self.stack.pop().unwrap() {
            Value::Dict(v) => v.clone(),
            other => panic!("expected number, got {:?}", other)
        };
    }

    fn pop_string(&mut self) -> String {
        return match self.stack.pop().unwrap() {
            Value::Identifier(v) => v,
            other => panic!("expected string, got {:?}", other)
        };
    }

    fn pop_string_u8(&mut self) -> Vec<u8> {
        return match self.stack.pop().unwrap() {
            Value::StringBytes(v) => v,
            other => panic!("expected string bytes, got {:?}", other)
        };
    }


    fn pop_array(&mut self) -> Vec<Box<Value>> {
        return match self.stack.pop().unwrap() {
            Value::Array(arr) => arr,
            other => panic!("expected array, got {:?}", other)
        };
    }

    fn curr_size(&self) -> f64 {
        let mm = self.text_state.size;
        return match mm {
            Some(vv) => vv,
            _ => 16.0,
        };
    }

    fn get_font_id(&self) -> String {
        return self.get_font().id.clone();
    }

    fn get_font(&self) -> &Font {
        return match self.text_state.font {
            None => panic!(),
            Some(ref other) => self.ctx.font_lib.get_font(&other),
        }
    }

    fn mk_message(&mut self, bytes: Vec<u8>) -> Message {
        let mut messages = vec![];

        for &byte in bytes.iter() {
            let effective = self.get_effective_ctm();
            let screen_x = effective.x();
            let screen_y = effective.y();
            let size = self.curr_size() * effective.y_scale().abs();

            let cwidth = (self.get_font().char_width(byte) * self.text_state.matrix.x_scale() * self.curr_size())/1000.0;

            messages.push(Message::DrawGlyph(GlyphInfo{
                x: screen_x,
                y: screen_y,
                byte: byte,
                size: size,
                font_id: self.get_font_id(),
                width: cwidth,
                colour: self.graphics_state.colour_nostroke.clone(),
            }));

            self.set_x(self.text_x() + cwidth);
        }
        return self.mk_message_block(messages);
    }

    fn mk_message_block(&mut self, msgs: Vec<Message>) -> Message {
        let msgs = msgs.into_iter().filter(|c| !matches!(c, Message::Noop)).collect();
        return Message::DrawBlock(msgs);
    }

    fn process_op(&mut self, tok: &Token) -> Message {
        match tok {
            Token::TmKeyword => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(self.pop_number());
                }
                mat.reverse();
                self.text_state.matrix = Matrix::vec6_to_matrix(&mat);
                return Message::Noop;
            },
            Token::TfKeyword => {
                let size = self.pop_number();
                let font = self.pop_string();
                self.text_state.font = Some(font);
                self.text_state.size = Some(size);
                return Message::Noop;
            },
            Token::TjKeyword => {
                // show one
                let str = self.pop_string_u8();
                return self.mk_message(str);
            },
            Token::TJKeyword => {
                // show one or mroe
                let arr = self.pop_array();
                let mut msgs = vec![];
                for val in arr {
                    let msg = match *val {
                        Value::Number(x) => {
                            self.move_x(x);
                            Message::Noop
                        },
                        Value::StringBytes(vec) => {
                            self.mk_message(vec)
                        },
                        other => panic!("unexpected value {:?}", other),
                    };
                    msgs.push(msg);
                }
                return self.mk_message_block(msgs);
            },
            Token::GSKeyword => {
                let _cs = self.pop_string();
                println!("TODO: implement gs keyword");
                return Message::Noop;
            },
            _ => panic!("Unexpected operator {:?}", tok),
        }
    }

    fn set_x(&mut self, x: f64) {
        self.text_state.matrix.set_x(x);
    }

    fn set_cs_nostroke(&mut self, cs_id: String) {
        self.graphics_state.cs_nostroke = Some(cs_id);
    }

    fn move_x(&mut self, x: f64) {
        let effective = self.get_effective_ctm();
        let x_scale = effective.x_scale();
        let old_x = self.text_state.matrix.x();
        self.text_state.matrix.set_x(
            old_x - (x * x_scale)/1000.0
        );
    }

    fn get_effective_ctm(&self) -> Matrix {
        let ctm = self.curr_ctm();
        return multiply_3d(&self.text_state.matrix, &ctm);
    }

    fn parse_value(&mut self) -> Value {
        let tok = self.next_token();
        return match tok {
            Token::Identifier(id) => Value::Identifier(id.to_string()),
            Token::Number(x) => Value::Number(x),
            Token::LBracket => self.parse_array(),
            Token::StringBytes(bytes) => Value::StringBytes(bytes),
            Token::AngleOpen => self.parse_dict(),
            _ => panic!("Unexpected token {:?}", tok),
        }
    }

    fn peek(&self) -> Token {
        return self.tokens[self.offset].clone();
    }

    fn parse_dict(&mut self) -> Value {
        let mut result = HashMap::new();
        assert!(matches!(self.next_token(), Token::AngleOpen));

        loop {
            let id_tok = self.next_token();
            match id_tok {
                Token::Identifier(id) => {
                    let value = self.parse_value();
                    result.insert(id, value);
                },
                Token::AngleClose => break,
                _ => panic!(),
            }
        }

        assert!(matches!(self.next_token(), Token::AngleClose));
        return Value::Dict(result);
    }

    fn parse_array(&mut self) -> Value {
        let mut arr = vec![];

        while !matches!(self.peek(), Token::RBracket) {
            assert!(!matches!(self.peek(), Token::LBracket));
            let expr = self.parse_value();
            arr.push(Box::new(expr));
        }
        self.next_token();
        return Value::Array(arr);
    }
}

#[cfg(test)]
mod tests {
use std::fs;

use crate::content::tokenizer::tokenize_stream;
use crate::fonts::FontLib;
use crate::pdf::ast::{ColourSpace, ColourSpaceLib};
use crate::pdf::parser::parse_tokens;
use crate::pdf::tokenizer::tokenize_pdf;

use super::*;

pub fn collect_messages(ctx: PageCtx, toks: Vec<Token>) -> (Vec<Message>, ContentStreamer) {
    let streamer = ContentStreamer::new(ctx, toks);
    let mut messages = vec![];
    let mut current = streamer;

    loop {
        if current.offset >= current.tokens.len() {
            break;
        }
        let msg = current.advance();
        if !matches!(msg, Message::Noop) {
            messages.push(msg);
        }
    }
    return (messages, current);
}

fn dummy_ctx() -> PageCtx {
    return PageCtx {
        height: 500.0,
        width: 500.0,
        font_lib: FontLib {
            id_to_font: HashMap::new(),
        },
        scale_factor: 1.0,
        cs_lib: ColourSpaceLib {
            id_to_cs: HashMap::new(),
        },
    };
}

fn dummy_font() -> Font {
    let ttf_file: Vec<u8> = fs::read("./test/inter.ttf").expect("woops");
    let ttf = fontdue::Font::from_bytes(ttf_file, fontdue::FontSettings::default()).unwrap();
    let font = Font {
        id: "F1".to_string(),
        name: "Inter".to_string(),
        widths: vec![100; 100],
        first_char: 32,
        ttf: ttf,
        encoding: Some("MacRomanEncoding".to_string()),
    };
    return font;
}

#[test]
fn saved_graphics() {
    let ctx = dummy_ctx();
    let vv = vec![
        Token::SaveGraphicsState,
        Token::RestoreGraphicsState,
        Token::SaveGraphicsState,
        Token::RestoreGraphicsState,
    ];
    let (messages, fstate) = collect_messages(ctx, vv);

    // Should all be no-ops
    assert_eq!(messages.len(), 0);
    assert_eq!(fstate.graphics_state.colour_nostroke, None);
    assert_eq!(fstate.graphics_state_stack.len(), 0);
}

#[test]
fn simple_colour() {
    let mut ctx = dummy_ctx();
    ctx.add_colourspace("Cs1".to_string(), ColourSpace { num_components: 1 });
    let toks = vec![
        Token::Identifier("Cs1".to_string()),
        Token::CsNoStroke,

        Token::Number(0.2),
        Token::SetColourNoStroke,
    ];

    let (messages, streamer) = collect_messages(ctx, toks);
    assert_eq!(messages.len(), 0);
    assert_eq!(streamer.graphics_state.cs_nostroke, Some("Cs1".to_string()));
    assert_eq!(streamer.graphics_state.colour_nostroke, Some(vec![0.2]));
}

fn glyph_char(msg: &Message) -> char {
    match msg {
        Message::DrawGlyph(info) => {
            return info.byte as char;
        },
        _ => {
            panic!("expected glyph");
        }
    }
}

#[test]
fn simple_text() {
    let mut ctx = dummy_ctx();
    ctx.add_font(dummy_font());

    let toks = vec![
        Token::BTKeyword,

        Token::Identifier("F1".to_string()),
        Token::Number(16.0),
        Token::TfKeyword,

        Token::StringBytes("hello".to_string().as_bytes().to_vec()),
        Token::TjKeyword,

        Token::ETKeyword,
    ];

    let (messages, fstate) = collect_messages(ctx, toks);
    assert_eq!(messages.len(), 1);

    match &messages[0] {
        Message::DrawBlock(msgs) => {
            assert_eq!(msgs.len(), 5); // "hello"
            assert!(msgs.iter().all(|x| matches!(x, Message::DrawGlyph(_))));
            assert_eq!(glyph_char(&msgs[0]), 'h');
            assert_eq!(glyph_char(&msgs[1]), 'e');
            assert_eq!(glyph_char(&msgs[2]), 'l');
            assert_eq!(glyph_char(&msgs[3]), 'l');
            assert_eq!(glyph_char(&msgs[4]), 'o');
        },
        _ => panic!("expected draw block"),
    }

    assert_eq!(fstate.state, State::TopLevel);
}

#[test]
fn streamer_state() {
    let ctx = dummy_ctx();
    let toks = vec![
        Token::BTKeyword,
    ];

    let (messages, fstate) = collect_messages(ctx, toks);
    assert_eq!(messages.len(), 0);
    assert_eq!(fstate.state, State::InText);
}


#[test]
fn sample_pdf() {
    let data: Vec<u8> = fs::read("./examples/samplepdf.pdf").expect("woops");
    let tokens = tokenize_pdf(&data);
    let ast = parse_tokens(&tokens);

    println!("-----------");

    let trailer = ast.get_trailer_dict();
    let root_ref = trailer.get("Root").unwrap();
    let root = ast.get_object(root_ref);
    let pages_ref = root.get("Pages");
    let pages = ast.get_object(pages_ref);
    let kids = pages.get("Kids");
    let vec = kids.get_vec();
    assert!(vec.len() >= 1);

    let page = ast.get_object(&vec[0]);
    let contents = page.get("Contents").deref(&ast);
    let ctx = ast.mk_page_ctx(page);
    let decoded_contents = contents.decode();
    let tokenized_contents = tokenize_stream(decoded_contents);

    let (messages, _) = collect_messages(ctx, tokenized_contents);
    assert!(messages.len() > 1);
}
}
