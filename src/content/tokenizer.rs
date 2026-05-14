use std::collections::HashMap;

use crate::tokenizer::Tokenizer;

#[derive(Clone, Debug, PartialEq)]

pub enum Token {
    Number(f64),
    Identifier(String),
    StringBytes(Vec<u8>),
    Array(Vec<Token>),
    Dict(HashMap<String,Token>),

    SaveGraphicsState,
    RestoreGraphicsState,
    Rect,
    W,
    WStar,
    N,
    CsStroke,
    CsNoStroke,
    SetColourStroke,
    SetColourNoStroke,
    Fill,
    FillStar,
    I,
    CmStroke,
    BT,
    Tm,
    Tf,
    Tj,
    TJ,
    ET,
    M,
    L,
    H,
    V,
    Y,
    C,
    BMC,
    BDC,
    EMC,
    GS,
    GNonStroke,
    GStroke,
    RGNonStroke,
    RGStroke,
    WLineWidth,
    LineCap,
    LineJoin,
    Stroke,
    CharSpacing,
    Do,

    Null,
}

struct ContentTokenizer {
    data: Vec<u8>,
    offset: usize,
}

impl Tokenizer<Token> for ContentTokenizer {
    fn token_from_word(&self, word: &str) -> Token {
        return match word {
            "SC" => Token::SetColourStroke,
            "sc" => Token::SetColourNoStroke,
            "i" => Token::I,
            "cs" => Token::CsNoStroke,
            "CS" => Token::CsStroke,
            "cm" => Token::CmStroke,
            "BT" => Token::BT,
            "Tm" => Token::Tm,
            "Tf" => Token::Tf,
            "Tj" => Token::Tj,
            "ET" => Token::ET,
            "TJ" => Token::TJ,
            "re" => Token::Rect,
            "m" => Token::M,
            "l" => Token::L,
            "h" => Token::H,
            "v" => Token::V,
            "c" => Token::C,
            "y" => Token::Y,
            "BDC" => Token::BDC,
            "EMC" => Token::EMC,
            "BMC" => Token::BMC,
            "gs" => Token::GS,
            "g" => Token::GNonStroke,
            "G" => Token::GStroke,
            "rg" => Token::RGNonStroke,
            "RG" => Token::RGStroke,
            "w" => Token::WLineWidth,
            "J" => Token::LineCap,
            "j" => Token::LineJoin,
            "S" => Token::Stroke,
            "Tc" => Token::CharSpacing,
            "Do" => Token::Do,
            "Q" => Token::RestoreGraphicsState,
            "q" => Token::SaveGraphicsState,
            "n" => Token::N,
            _ => {
                println!("missed word: `{}`", word);
                panic!();
            }
        }
    }

    fn peek_u8(&self) -> u8 { return self.data[self.offset]; }
    fn step_ahead(&mut self) { self.offset += 1; }
    fn has_next(&self) -> bool { return self.offset < self.data.len(); }
}

impl ContentTokenizer {
    fn new(data: Vec<u8>) -> Self {
        return ContentTokenizer { data, offset: 0 };
    }

    fn lex_w(&mut self) -> Token {
        self.eat_char('W');
        if self.peek() == '*' {
            self.eat_char('*');
            return Token::WStar;
        } else {
            return Token::W;
        }
    }

    fn lex_f(&mut self) -> Token {
        self.eat_char('f');
        if self.peek() == '*' {
            self.eat_char('*');
            return Token::FillStar;
        } else {
            return Token::Fill;
        }
    }

    fn lex_identifier(&mut self) -> Token {
        assert_eq!(self.lex_char(), '/');
        let id = self.lex_word();
        return Token::Identifier(id);
    }

    fn lex_string(&mut self) -> Token {
        assert_eq!(self.lex_char(), '(');

        let mut bytes = vec![];
        let mut depth = 1;
        while depth > 0 && self.offset < self.data.len() {
            let mm = self.lex_u8();
            match mm as char {
                '\\' => {
                    // consume \(, \), \\, \n, etc.
                    bytes.push(mm);
                    assert!(self.has_next());
                    bytes.push(self.lex_u8());
                },
                ')' => {
                    depth -= 1;
                    if depth > 0 { bytes.push(mm); }
                },
                '(' => { depth += 1; bytes.push(mm); },
                _   => bytes.push(mm),
            }
        }
        return Token::StringBytes(bytes);
    }

    fn lex_keyword(&mut self) -> Token {
        let word = self.lex_word();
        return self.token_from_word(&word);
    }

    fn lex_number(&mut self) -> Token {
        let number = self.parse_next_number();
        return Token::Number(number);
    }

    fn lex_null(&mut self) -> Token {
        assert_eq!(self.lex_char(), '\0');
        return Token::Null;
    }

    fn lex_whitespace(&mut self) {
        while self.peek().is_whitespace() {
            self.lex_char();
        }
    }

    fn lex_array(&mut self) -> Token {
        assert_eq!(self.lex_char(), '[');
        let mut arr = vec![];
        self.lex_whitespace();
        while !matches!(self.peek(), ']') {
            assert!(!matches!(self.peek(), '['));
            arr.push(self.lex_next_value());
            self.lex_whitespace();
        }
        assert_eq!(self.lex_char(), ']');
        return Token::Array(arr);
    }

    fn lex_dict(&mut self) -> Token {
        let mut result = HashMap::new();
        assert_eq!(self.lex_char(), '<');
        assert_eq!(self.lex_char(), '<');

        self.lex_whitespace();

        while !matches!(self.peek(), '>') {
            assert_eq!(self.lex_char(), '/');
            let id = self.lex_word();
            let value = self.lex_next_value();
            self.lex_whitespace();
            result.insert(id, value);
        }
        assert_eq!(self.lex_char(), '>');
        assert_eq!(self.lex_char(), '>');
        return Token::Dict(result);
    }

    fn peek_at(&self, n: usize) -> Option<char> {
        if self.offset + n > self.data.len() {
            return None;
        }
        return Some(self.data[self.offset + n] as char);
    }

    fn lex_next_value(&mut self) -> Token {
        self.lex_whitespace();
        return match self.peek() {
            'W' => self.lex_w(),
            'f' => self.lex_f(),
            '/' => self.lex_identifier(),
            '(' => self.lex_string(),
            '\0' => self.lex_null(),
            '[' => self.lex_array(),
            '<' if self.peek_at(1) == Some('<') => self.lex_dict(),
            c if c.is_numeric() || c == '-' => self.lex_number(),
            _ => self.lex_keyword(),
        };
    }

    fn run(&mut self) -> Vec<Token>{
        self.offset = 0;
        let mut toks = vec![];

        while self.has_next() {
            if self.peek().is_whitespace() {
                self.lex_char();
                continue;
            }
            toks.push(self.lex_next_value());
        }
        return toks;
    }
}

pub fn tokenize_stream(str: Vec<u8>) -> Vec<Token> {
    let bytes = str.iter().copied().collect();
    return ContentTokenizer::new(bytes).run();
}

// TODO: add tests for hex numbers <...>

#[cfg(test)]
mod tests {
    use crate::test_consts::tests::SAMPLE_PDF_STREAM;
    use super::*;

    fn str_bytes(str: &str) -> Vec<u8> {
        return str.as_bytes().to_vec();
    }

    fn run_tokenizer(str: &str) -> Vec<Token> {
        return tokenize_stream(str_bytes(str));
    }

    #[test]
    fn simple_seqs() {
        let actual = run_tokenizer("
        q Q q Q q Q
        ");
        let expected = vec![
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn escaped_strings() {
        let actual = run_tokenizer("
        (\\(hello\\))
        ");
        let expected = vec![
            Token::StringBytes(str_bytes("\\(hello\\)")),
        ];
        assert_eq!(actual, expected);

        let actual = run_tokenizer("
        (hello\\n)
        ");
        let expected = vec![
            Token::StringBytes(str_bytes("hello\\n")),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn w_keywords() {
        let actual = run_tokenizer("q Q W q Q");
        let expected = vec![
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
            Token::W,
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
        ];
        assert_eq!(actual, expected);

        let actual = run_tokenizer("q Q W* q Q");
        let expected = vec![
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
            Token::WStar,
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn sample_pdf() {
        // Just check that it doesn't crash
        run_tokenizer(SAMPLE_PDF_STREAM);
    }
}
