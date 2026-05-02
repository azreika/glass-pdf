use crate::content::ContentToken;

use std::collections::HashMap;

use iced::{Color, Element};
use iced;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Length, Point, Renderer, Theme};

pub fn view_contents(tokens: &Vec<ContentToken>) {
    let mut parser = Parser { tokens: tokens.clone(), offset: 0, output: HashMap::new(), text_state: TextState::new() };
    parser.parse_program();
}


struct Parser {
    tokens: Vec<ContentToken>,
    offset: usize,
    output: HashMap<i32, TextInfo>,
    text_state: TextState,
}

#[derive(Debug, Clone)]
enum Value {
    Number(f64),
    Identifier(String),
    Array (Vec<Box<Value>>),
}

impl Value {
    fn value(&self) -> f64 {
        return match self {
            Value::Number(v) => *v,
            _ => panic!(),
        }
    }

    fn str(&self) -> String {
        return match self {
            Value::Identifier(v) => v.clone(),
            Value::Number(v) => v.to_string(),
            _ => {
                panic!();
            },
        }
    }

    fn arr(&self) -> Vec<Box<Value>> {
        return match self {
            Value::Array(arr) => arr.clone().to_vec(),
            _ => {
                panic!();
            },
        }
    }
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

        println!("--- Text ---");
        println!("--- --- ---");

        for (_, info) in text_vec.iter() {
            println!("{}", info.txt);
        }

        let output = self.output.clone();
        iced::application(
            move || (Viewer { output: output.clone() }, iced::Task::none()),
            Viewer::update,
            Viewer::view
        ).run().unwrap();
    }

    fn next_token(&mut self) -> ContentToken {
        let tok = self.tokens[self.offset].clone();
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

    fn pop_number(stack: &mut Vec<Value>) -> f64 {
        return stack.pop().unwrap().value();
    }

    fn pop_string(stack: &mut Vec<Value>) -> String {
        return stack.pop().unwrap().str();
    }

    fn pop_array(stack: &mut Vec<Value>) -> Vec<Box<Value>> {
        return stack.pop().unwrap().arr();
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
        let entry = self.output.entry(self.y()).or_insert(TextInfo {
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
                for val in arr.iter() {
                    if matches!(**val, Value::Number(_)) {
                        // self.output += &format!("<space_{:?}>", val.value());
                    } else {
                        assert!(matches!(**val, Value::Identifier(_)));
                        self.add_output(&val.str());
                    }
                }
            }
            _ => panic!(),
        }
    }

    fn parse_parens(&mut self) -> Value {
        let mut stack = vec![];

        let mut tok = self.next_token();
        while !matches!(tok, ContentToken::RParens) {
            if matches!(tok, ContentToken::LParens) {
                let expr = self.parse_parens();
                stack.push(expr);
            } else if matches!(tok, ContentToken::Identifier(_)) {
                stack.push(Value::Identifier(tok.ident()));
            } else {
                println!("Unhandled op in parens parse: {:?}", tok);
                println!("{:?}", stack);
                panic!();
            }
            tok = self.next_token();
        }

        assert_eq!(stack.len(), 1);
        return stack[0].clone();
    }

    fn parse_array(&mut self) -> Value {
        let mut arr = vec![];

        let mut tok = self.next_token();
        while !matches!(tok, ContentToken::RBracket) {
            assert!(!matches!(tok, ContentToken::LBracket));
            if matches!(tok, ContentToken::Number(_)) {
                let expr = tok.value();
                arr.push(Box::new(Value::Number(expr)));
            } else if matches!(tok, ContentToken::LParens) {
                let expr = self.parse_parens();
                arr.push(Box::new(expr));
            } else {
                println!("Unhandled op in bracket parse: {:?}", tok);
                println!("{:?}", arr);
                panic!()
            }
            tok = self.next_token();
        }
        return Value::Array(arr);
    }

    fn parse_text_block(&mut self) {
        assert!(matches!(self.next_token(), ContentToken::BTKeyword));
        let mut stack = vec![];

        let mut tok = self.next_token();
        while !matches!(tok, ContentToken::ETKeyword) {
            if self.is_operator(&tok) {
                self.process_op(&mut stack, &tok);
            } else if matches!(tok, ContentToken::Identifier(_)) {
                stack.push(Value::Identifier(tok.ident()));
            } else if matches!(tok, ContentToken::LParens)   {
                stack.push(self.parse_parens());
            } else if matches!(tok, ContentToken::LBracket) {
                stack.push(self.parse_array());
            } else {
                if !(matches!(tok, ContentToken::Number(_))) {
                    println!("Unhandled op in text block parse: {:?}", tok);
                    println!("{:?}", stack);
                    panic!();
                }
                stack.push(Value::Number(tok.value()));
            }
            tok = self.next_token();
        }

        assert!(stack.is_empty());
        self.reset_text_state();
    }
}


#[derive(Clone, Debug)]
struct TextInfo {
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

        for (pos, info) in self.output.iter() {
            let mut frame = Frame::new(renderer, bounds.size());
            let mut txt = canvas::Text::from(
                info.txt.clone()
            );
            txt.position = Point::new(self.padding_x, *pos as f32 + self.padding_y);
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
