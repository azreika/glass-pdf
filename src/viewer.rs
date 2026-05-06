use crate::content_tokenizer::ContentToken;

use std::collections::HashMap;

use iced::{Color, Element};
use iced;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Length, Point, Renderer, Theme};

use crate::ast::FontLib;

pub fn view_contents(font_lib: &FontLib, tokens: &Vec<ContentToken>) {
    let mut parser = Parser::new(font_lib.clone(), tokens.clone());
    parser.parse_program();
}

struct Parser {
    tokens: Vec<ContentToken>,
    offset: usize,
    output: HashMap<i32, TextInfo>,
    text_state: TextState,
    font_lib: FontLib,
}

impl Parser {
    fn new(font_lib: FontLib, tokens: Vec<ContentToken>) -> Self {
        return Parser {
            tokens,
            offset: 0,
            output: HashMap::new(),
            text_state: TextState::new(),
            font_lib,
        };
    }

}

#[derive(Debug, Clone)]
enum Value {
    Number(f64),
    Identifier(String),
    Array (Vec<Box<Value>>),
}

struct Viewer {
    output: HashMap<i32, TextInfo>
}

struct Message {

}

impl Default for Viewer {
    fn default() -> Self {
        return Viewer { output: HashMap::new() }
    }
}

impl Viewer {
    fn update(&mut self, _: Message) {

    }

    fn view(&self) -> Element<'_, Message> {
        return Canvas::new(Page {
            padding_x: 40.0,
            padding_y: 20.0,
            output: self.output.clone()
        })
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
}

impl Parser {
    fn reset_text_state(&mut self) {
        self.text_state = TextState::new();
    }

    fn parse_program(&mut self) {
        while self.offset < self.tokens.len() {
            let tok = &self.tokens[self.offset];
            if matches!(tok, ContentToken::BTKeyword) {
                self.parse_text_block();
            }
            self.offset += 1;
        }

        let mut text_vec: Vec<(&i32, &TextInfo)> = self.output.iter().collect();
        text_vec.sort_by(|a,b| a.0.cmp(b.0));
        text_vec.reverse();

        let output = self.output.clone();
        iced::application(
            move || (Viewer { output: output.clone() }, iced::Task::none()),
            Viewer::update,
            Viewer::view
        ).run().unwrap();
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

    fn y(&self) -> i32 {
        return self.text_state.matrix[7] as i32;
    }

    fn x(&self) -> i32 {
        return self.text_state.matrix[6] as i32;
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

    fn add_output(&mut self, str: &str) {
        let x_scale = self.text_state.matrix[0] as f32;
        let init_size = self.curr_size();
        let size = init_size * x_scale;
        let x_pos = self.x();
        let entry = self.output.entry(self.y()).or_insert(TextInfo {
            x: x_pos,
            txt: String::new(),
            size: size,
        });
        entry.txt += str;
    }

    fn process_op(&mut self, stack: &mut Vec<Value>, tok: &ContentToken) {
        match tok {
            ContentToken::TmKeyword => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(Self::pop_number(stack));
                }
                mat.reverse();
                self.text_state.set_matrices(&mat);
            },
            ContentToken::TfKeyword => {
                let size = Self::pop_number(stack);
                let font = Self::pop_string(stack);
                self.text_state.font = Some(font);
                self.text_state.size = Some(size);
            },
            ContentToken::TjKeyword => {
                // show one
                let str = Self::pop_string(stack);
                self.add_output(&str);
            },
            ContentToken::TJKeyword => {
                // show one or mroe
                let arr = Self::pop_array(stack);
                for val in arr {
                    match *val {
                        Value::Number(_) => {},
                        Value::Identifier(id) => self.add_output(&id.to_string()),
                        other => panic!("unexpected array value {:?}", other),
                    }
                }
            }
            _ => panic!("Unexpected operator {:?}", tok),
        }
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

    fn parse_text_block(&mut self) {
        assert!(matches!(self.next_token(), ContentToken::BTKeyword));
        let mut stack = vec![];

        while !matches!(self.peek(), ContentToken::ETKeyword) {
            if self.is_operator(&self.peek()) {
                let tok = self.next_token();
                self.process_op(&mut stack, &tok);
                continue;
            }
            stack.push(self.parse_value());
        }
        self.next_token();

        assert!(stack.is_empty());
        self.reset_text_state();
    }
}


#[derive(Clone, Debug)]
struct TextInfo {
    x: i32,
    txt: String,
    size: f32,
}

struct Page {
    padding_x: f32,
    padding_y: f32,
    output: HashMap<i32, TextInfo>
}

impl <Message> canvas::Program<Message> for Page {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {

        let mut geom: Vec<Geometry> = vec![];

        // outer rectangle
        let mut f1 = Frame::new(renderer, bounds.size());
        let outer_rect = canvas::Path::rectangle(Point { x: 0.0, y: 0.0 }, bounds.size());
        f1.fill(&outer_rect, Color::from_rgb(0.2, 0.5, 1.0));
        geom.push(f1.into_geometry());

        // inner rectangle
        let mut f2 = Frame::new(renderer, bounds.size());
        let inner_size = iced::Size {
            width: bounds.size().width - self.padding_x*2.0,
            height: bounds.size().height - self.padding_y*2.0,
        };

        let inner_rect = canvas::Path::rectangle(Point { x: self.padding_x, y: self.padding_y}, inner_size);
        f2.fill(&inner_rect, Color::from_rgb(1.0, 1.0, 1.0));
        geom.push(f2.into_geometry());

        for (pos, info) in self.output.iter() {
            let mut frame = Frame::new(renderer, bounds.size());
            let mut txt = canvas::Text::from(
                info.txt.clone()
            );
            txt.position = Point::new(self.padding_x + info.x as f32, *pos as f32 + self.padding_y);
            txt.size = info.size.into();
            frame.fill_text(txt);
            geom.push(frame.into_geometry());
        }

        return geom;
    }
}

struct TextState {
    matrix: Vec<f64>,
    line_matrix: Vec<f64>,
    font: Option<String>,
    size: Option<f64>,
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
