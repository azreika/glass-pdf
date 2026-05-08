use std::collections::HashMap;

use iced::futures::stream;
use crate::content::tokenizer::ContentToken;
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
    tokens: Vec<ContentToken>,
    stack: Vec<Value>,
    offset: usize,

    text_state: TextState,

    graphics_state: GraphicsState,
    graphics_state_stack: Vec<GraphicsState>,

    ctx: PageCtx,
}


impl ContentStreamer {
    fn new(ctx: PageCtx, tokens: Vec<ContentToken>) -> Self {
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

    pub fn stream_content(ctx: PageCtx, toks: Vec<ContentToken>) -> impl iced::futures::Stream<Item=Message> {
        let p = Self::new(ctx, toks);
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

                let value = self.parse_value();
                self.stack.push(value);
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
            matches!(tok, ContentToken::TJKeyword) ||
            matches!(tok, ContentToken::GSKeyword);
    }

    fn num_colour_components(&self) -> u8 {
        let cs = self.graphics_state.cs_nostroke.as_ref().unwrap();
        return self.ctx.cs_lib.num_components(cs.to_string());
    }

    fn set_colour_nostroke(&mut self, vv: Vec<f64>) {
        self.graphics_state.colour_nostroke = Some(vv);
    }

    fn is_main_operator(&self, tok: &ContentToken) -> bool {
        return
            matches!(tok, ContentToken::SaveGraphicsState) ||
            matches!(tok, ContentToken::RestoreGraphicsState) ||
            matches!(tok, ContentToken::RectKeyword) ||
            matches!(tok, ContentToken::WKeyword) ||
            matches!(tok, ContentToken::NKeyword) ||
            matches!(tok, ContentToken::CsNoStroke) ||
            matches!(tok, ContentToken::SetColourNoStroke) ||
            matches!(tok, ContentToken::Fill) ||
            matches!(tok, ContentToken::IKeyword) ||
            matches!(tok, ContentToken::CmStroke) ||
            matches!(tok, ContentToken::MKeyword) ||
            matches!(tok, ContentToken::HKeyword) ||
            matches!(tok, ContentToken::LKeyword) ||
            matches!(tok, ContentToken::VKeyword) ||
            matches!(tok, ContentToken::YKeyword) ||
            matches!(tok, ContentToken::EMCKeyword) ||
            matches!(tok, ContentToken::BMCKeyword | ContentToken::WStarKeyword) ||
            matches!(tok, ContentToken::GSKeyword)
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
                let _height = self.pop_number();
                let _width = self.pop_number();
                let _y = self.pop_number();
                let _x = self.pop_number();
                println!("TODO: implement rectangle thing");
                return Message::Noop;
            },
            ContentToken::WKeyword | ContentToken::WStarKeyword => {
                // Clipping Path Operator
                println!("TODO: implement clipping path operator W/W*");
                return Message::Noop;
            },
            ContentToken::NKeyword => {
                // Clipping Path Operator - end path object without filling it
                println!("TODO: implement clipping path operator N");
                return Message::Noop;
            },
            ContentToken::CsNoStroke => {
                let cs = self.pop_string();
                self.set_cs_nostroke(cs);
                return Message::Noop;
            },
            ContentToken::GSKeyword => {
                let _cs = self.pop_string();
                println!("TODO: implement gs keyword");
                return Message::Noop;
            },
            ContentToken::SetColourNoStroke => {
                let num_components = self.num_colour_components();
                let mut vv = vec![];
                for _ in 0..num_components {
                    vv.push(self.pop_number());
                }
                vv.reverse();
                self.set_colour_nostroke(vv);
                return Message::Noop;
            },
            ContentToken::Fill => {
                println!("TODO: implement colour space operator fill");
                return Message::Noop;
            },
            ContentToken::IKeyword => {
                println!("TODO: implement colour space operator flatness I");
                let _flatness = self.pop_number();
                return Message::Noop;
            },
            ContentToken::CmStroke => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(self.pop_number());
                }
                mat.reverse();
                let mat = Matrix::vec6_to_matrix(&mat);
                let result = multiply_3d(self.curr_ctm(), &mat);
                self.graphics_state.ctm = result;
                return Message::Noop;
            },
            ContentToken::MKeyword | ContentToken::LKeyword => {
                let _y = self.pop_number();
                let _x = self.pop_number();
                println!("TODO: implement M and L keyword");
                return Message::Noop;
            },
            ContentToken::VKeyword | ContentToken::YKeyword => {
                let _x1 = self.pop_number();
                let _x2 = self.pop_number();
                let _x3 = self.pop_number();
                let _x4 = self.pop_number();
                println!("TODO: implement V and Y keyword");
                return Message::Noop;
            },
            ContentToken::HKeyword => {
                println!("TODO: implement H keyword");
                return Message::Noop;
            },
            ContentToken::BMCKeyword => {
                let _dict = self.pop_dict();
                println!("TODO: implement BMC keyword");
                return Message::Noop;
            }
            ContentToken::EMCKeyword => {
                println!("TODO: implement EMC keyword");
                return Message::Noop;
            }
            _ => panic!(),
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

    fn process_op(&mut self, tok: &ContentToken) -> Message {
        match tok {
            ContentToken::TmKeyword => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(self.pop_number());
                }
                mat.reverse();
                self.text_state.matrix = Matrix::vec6_to_matrix(&mat);
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
                let str = self.pop_string_u8();
                return self.mk_message(str);
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
            ContentToken::GSKeyword => {
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
            ContentToken::Identifier(id) => Value::Identifier(id.to_string()),
            ContentToken::Number(x) => Value::Number(x),
            ContentToken::LParens => self.parse_parens(),
            ContentToken::LBracket => self.parse_array(),
            ContentToken::StringBytes(bytes) => Value::StringBytes(bytes),
            ContentToken::AngleOpen => self.parse_dict(),
            _ => panic!("Unexpected token {:?}", tok),
        }
    }

    fn peek(&self) -> ContentToken {
        return self.tokens[self.offset].clone();
    }

    fn parse_dict(&mut self) -> Value {
        let mut result = HashMap::new();
        assert!(matches!(self.next_token(), ContentToken::AngleOpen));

        loop {
            let id_tok = self.next_token();
            match id_tok {
                ContentToken::Identifier(id) => {
                    let value = self.parse_value();
                    result.insert(id, value);
                },
                ContentToken::AngleClose => break,
                _ => panic!(),
            }
        }

        assert!(matches!(self.next_token(), ContentToken::AngleClose));
        return Value::Dict(result);
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

#[cfg(test)]
mod tests {

    use crate::fonts::FontLib;
    use crate::pdf::ast::ColourSpaceLib;

    use super::*;
    use futures::executor::block_on;
    use futures::StreamExt;

    fn collect_messages(ctx: PageCtx, tokens: Vec<ContentToken>) -> Vec<Message> {
        let stream = ContentStreamer::stream_content(ctx, tokens);
        futures::pin_mut!(stream);
        let messages: Vec<Message> = block_on(stream.collect());
        // flatten DrawBlocks
        let mut flat = vec![];
        for msg in messages {
            flatten_message(msg, &mut flat);
        }
        return flat;
    }

    fn flatten_message(msg: Message, out: &mut Vec<Message>) {
        match msg {
            Message::DrawBlock(msgs) => {
                for m in msgs {
                    flatten_message(m, out);
                }
            }
            other => out.push(other),
        }
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

    #[test]
    fn saved_graphics() {
        let ctx = dummy_ctx();
        let vv = vec![
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
        ];
        let messages = collect_messages(ctx, vv);

        // Should all be no-ops
        assert_eq!(messages.len(), 0);
    }
}
