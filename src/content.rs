use std::collections::HashMap;

use iced::{Element, Color};
use iced;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Length, Point, Renderer, Theme};

#[derive(Clone, Debug)]
enum ContentToken {
    SaveGraphicsState,
    RestoreGraphicsState,
    Number(f64),
    RectKeyword,
    WKeyword,
    WStarKeyword,
    NKeyword,
    Identifier(String),
    CsStroke,
    CsNoStroke,
    EndCsStroke,
    EndCsNoStroke,
    Fill,
    IKeyword,
    CmStroke,
    BTKeyword,
    TmKeyword,
    TfKeyword,
    LParens,
    RParens,
    TjKeyword,
    TJKeyword,
    ETKeyword,
    LBracket,
    RBracket,
    Null,
}

impl ContentToken {
    fn value(&self) -> f64 {
        return match self {
            ContentToken::Number(v) => *v,
            _ => panic!(),
        }
    }

    fn ident(&self) -> String {
        return match self {
            ContentToken::Identifier(v) => v.clone(),
            _ => panic!(),
        }
    }
}

struct Glyph {
    txt: String
}

impl <Message> canvas::Program<Message> for Glyph {
    type State = ();

     fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let txt = canvas::Text::from(self.txt.clone());
        frame.fill_text(txt);
        return vec![frame.into_geometry()]
    }
}

struct ContentTokenizer {
    data: Vec<char>,
    offset: usize,
    missed_words: u32,
}

impl ContentTokenizer {
    fn peek(&self) -> char {
        return self.data[self.offset];
    }

    fn eat_next(&mut self) -> char {
        let result = self.data[self.offset];
        self.offset += 1;
        return result;
    }

    fn lex_number(&mut self) -> f64 {
        let mut chars = vec![];
        if self.peek_is('-') {
            chars.push(self.eat_next());
        }
        let mut cc = self.peek();
        while cc == '.' || cc.is_numeric() {
            chars.push(self.eat_next());
            cc = self.peek();
        }
        let str: String = chars.iter().collect();
        return str.parse().unwrap();
    }

    fn peek_is(&self, c: char) -> bool {
        return self.peek() == c;
    }

    fn eat_char(&mut self, c: char) {
        assert!(self.peek_is(c));
        self.offset += 1;
    }

    fn lex_identifier(&mut self) -> ContentToken {
        let mut chars = vec![];
        while is_identifier_char(self.peek()) {
            chars.push(self.lex_char());
        }
        let str = chars.iter().collect();
        return ContentToken::Identifier(str);
    }

    fn lex_char(&mut self) -> char {
        let result = self.peek();
        self.offset += 1;
        return result;
    }

    fn lex_word(&mut self) -> String {
        if !is_identifier_char(self.peek()) {
            return self.eat_next().to_string();
        }
        let mut chars = vec![];
        while is_identifier_char(self.peek()) {
            chars.push(self.lex_char());
        }
        let str = chars.iter().collect();
        return str;
    }

    fn token_from_word(&mut self, word: &str) -> ContentToken {
        return match word {
            "SC" => ContentToken::EndCsNoStroke,
            "sc" => ContentToken::EndCsStroke,
            "f" => ContentToken::Fill,
            "i" => ContentToken::IKeyword,
            "cs" => ContentToken::CsStroke,
            "CS" => ContentToken::CsNoStroke,
            "cm" => ContentToken::CmStroke,
            "BT" => ContentToken::BTKeyword,
            "Tm" => ContentToken::TmKeyword,
            "Tf" => ContentToken::TfKeyword,
            "(" => ContentToken::LParens,
            "Tj" => ContentToken::TjKeyword,
            "ET" => ContentToken::ETKeyword,
            "[" => ContentToken::LBracket,
            "]" => ContentToken::RBracket,
            "TJ" => ContentToken::TJKeyword,
            "re" => ContentToken::RectKeyword,
            _ => {
                println!("missed word: `{}`", word);
                panic!();
                // println!("Got to {} out of {} ({}%)", self.offset, self.data.len(), (self.offset as f64 * 100.0 / self.data.len() as f64) );
            }
        }
    }
}

fn is_identifier_char(c: char) -> bool {
    return c.is_alphanumeric() || matches!(c, '.' | '-' | '+');
}

fn tokenize_stream(str: String) -> Vec<ContentToken> {
    let mut vv = vec![];
    println!("{}", str);

    let mut tokenizer = ContentTokenizer { data: str.chars().collect(), offset: 0, missed_words: 0 };
    while tokenizer.offset < tokenizer.data.len() {
        let cc = tokenizer.peek();
        if cc.is_whitespace() {
            tokenizer.eat_next();
        } else if cc == '\0' {
            tokenizer.eat_next();
            vv.push(ContentToken::Null);
        } else if cc == 'q' {
            tokenizer.eat_next();
            vv.push(ContentToken::SaveGraphicsState);
        } else if cc == 'Q' {
            tokenizer.eat_next();
            vv.push(ContentToken::RestoreGraphicsState);
        } else if cc.is_numeric() || cc == '.' || cc == '-' {
            let num = tokenizer.lex_number();
            vv.push(ContentToken::Number(num));
        } else if cc == 'W' {
            tokenizer.eat_char('W');
            if tokenizer.peek() == '*' {
                tokenizer.eat_char('*');
                vv.push(ContentToken::WStarKeyword);
            } else {
                vv.push(ContentToken::WKeyword);
            }
        } else if cc == 'n' {
            tokenizer.eat_char('n');
            vv.push(ContentToken::NKeyword);
        } else if cc == '/' {
            tokenizer.eat_char('/');
            let id = tokenizer.lex_identifier();
            vv.push(id);
        } else {
            let word = tokenizer.lex_word();
            let tok = tokenizer.token_from_word(&word);
            vv.push(tok.clone());

            if matches!(tok, ContentToken::LParens) {
                let mut chars = vec![];
                let mut depth = 1;
                while depth > 0 && tokenizer.offset < tokenizer.data.len() {
                    let mm = tokenizer.eat_next();
                    match mm {
                        '\\' => {
                            chars.push(mm);
                            if tokenizer.offset < tokenizer.data.len() {
                                chars.push(tokenizer.eat_next()); // consume \(, \), \\, \n, etc.
                            }
                        },
                        ')' => {
                            depth -= 1;
                            if depth > 0 { chars.push(mm); }
                        },
                        '(' => { depth += 1; chars.push(mm); },
                        _   => chars.push(mm),
                    }
                }
                let str = chars.iter().collect();
                vv.push(ContentToken::Identifier(str));
                vv.push(ContentToken::RParens);
            }
        }
    }

    println!("Numebr of missed words: {}", tokenizer.missed_words);

    return vv;
}

pub fn parse_stream(result: String) {
    let tokens = tokenize_stream(result);
    let mut parser = Parser { tokens: tokens, offset: 0, output: HashMap::new(), text_state: TextState::new() };
    parser.parse_program();
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

struct Parser {
    tokens: Vec<ContentToken>,
    offset: usize,
    output: HashMap<i32, String>,
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

}

struct Message {

}

impl Default for Viewer {
    fn default() -> Self {
        return Viewer {}
    }
}

impl Viewer {
    fn update(&mut self, _: Message) {

    }

    fn view(&self) -> Element<'_, Message> {
        return Canvas::new(Glyph {
            txt: "hello".to_string(),
        }).into();
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

        let mut text_vec: Vec<(&i32, &String)> = self.output.iter().collect();
        text_vec.sort_by(|a,b| a.0.cmp(b.0));
        text_vec.reverse();

        println!("--- Text ---");
        println!("--- --- ---");

        for (_, txt) in text_vec.iter() {
            println!("{}", txt);
        }
        iced::run(Viewer::update, Viewer::view).unwrap();
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

    fn add_output(&mut self, str: &str) {
        *self.output.entry(self.y()).or_insert("".to_string()) += str;
    }

    fn process_op(&mut self, stack: &mut Vec<Value>, tok: &ContentToken) {
        match tok {
            ContentToken::TmKeyword => {
                let mut mat = vec![];
                for _ in 0..6 {
                    mat.push(Self::pop_number(stack));
                }
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
