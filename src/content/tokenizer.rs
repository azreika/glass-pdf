use crate::tokenizer::Tokenizer;

#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Number(f64),
    Identifier(String),
    StringBytes(Vec<u8>),

    SaveGraphicsState,
    RestoreGraphicsState,
    RectKeyword,
    WKeyword,
    WStarKeyword,
    NKeyword,
    CsStroke,
    CsNoStroke,
    SetColourStroke,
    SetColourNoStroke,
    Fill,
    IKeyword,
    CmStroke,
    BTKeyword,
    TmKeyword,
    TfKeyword,
    TjKeyword,
    TJKeyword,
    ETKeyword,
    LBracket,
    RBracket,
    MKeyword,
    LKeyword,
    HKeyword,
    VKeyword,
    YKeyword,
    CKeyword,
    AngleOpen,
    AngleClose,
    BMCKeyword,
    EMCKeyword,
    GSKeyword,
    GNonStroke,
    GStroke,
    RGNonStroke,
    RGStroke,
    Star,
    WLineWidth,
    LineCap,
    LineJoin,
    Stroke,
    CharSpacing,
    DoKeyword,

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
            "f" => Token::Fill,
            "i" => Token::IKeyword,
            "cs" => Token::CsNoStroke,
            "CS" => Token::CsStroke,
            "cm" => Token::CmStroke,
            "BT" => Token::BTKeyword,
            "Tm" => Token::TmKeyword,
            "Tf" => Token::TfKeyword,
            "Tj" => Token::TjKeyword,
            "ET" => Token::ETKeyword,
            "[" => Token::LBracket,
            "]" => Token::RBracket,
            "TJ" => Token::TJKeyword,
            "re" => Token::RectKeyword,
            "m" => Token::MKeyword,
            "l" => Token::LKeyword,
            "h" => Token::HKeyword,
            "v" => Token::VKeyword,
            "c" => Token::CKeyword,
            "y" => Token::YKeyword,
            "<" => Token::AngleOpen,
            ">" => Token::AngleClose,
            "BDC" => Token::BMCKeyword,
            "EMC" => Token::EMCKeyword,
            "BMC" => Token::BMCKeyword,
            "gs" => Token::GSKeyword,
            "g" => Token::GNonStroke,
            "G" => Token::GStroke,
            "rg" => Token::RGNonStroke,
            "RG" => Token::RGStroke,
            "*" => Token::Star,
            "w" => Token::WLineWidth,
            "J" => Token::LineCap,
            "j" => Token::LineJoin,
            "S" => Token::Stroke,
            "Tc" => Token::CharSpacing,
            "Do" => Token::DoKeyword,
            "Q" => Token::RestoreGraphicsState,
            "q" => Token::SaveGraphicsState,
            "n" => Token::NKeyword,
            _ => {
                println!("missed word: `{}`", word);
                panic!();
            }
        }
    }

    fn peek_u8(&self) -> u8 {
        return self.data[self.offset];
    }

    fn step_ahead(&mut self) {
        self.offset += 1;
    }

    fn has_next(&self) -> bool {
        return self.offset < self.data.len();
    }
}

impl ContentTokenizer {
    fn new(data: Vec<u8>) -> Self {
        return ContentTokenizer { data, offset: 0 };
    }

    fn lex_w(&mut self) -> Token {
        self.eat_char('W');
        if self.peek() == '*' {
            self.eat_char('*');
            return Token::WStarKeyword;
        } else {
            return Token::WKeyword;
        }
    }

    fn lex_identifier(&mut self) -> Token {
        let id = self.lex_word();
        return Token::Identifier(id);
    }

    fn lex_string(&mut self) -> Token {
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

    fn run(&mut self) -> Vec<Token>{
        self.offset = 0;
        let mut toks = vec![];

        while self.offset < self.data.len() {
            match self.peek() {
                'W' => { toks.push(self.lex_w())},
                '/' => { self.lex_char(); toks.push(self.lex_identifier()); },
                '(' => { self.lex_char(); toks.push(self.lex_string()); },
                '\0' => { self.lex_char(); toks.push(Token::Null); }
                c if c.is_whitespace() => { self.lex_char(); },
                c if c.is_numeric() || c == '-' => { toks.push(self.lex_number()); },
                _ => { toks.push(self.lex_keyword()); }
            };
        }
        return toks;
    }
}

pub fn tokenize_stream(str: Vec<u8>) -> Vec<Token> {
    let bytes = str.iter().copied().collect();
    return ContentTokenizer::new(bytes).run();
}

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
            Token::WKeyword,
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
        ];
        assert_eq!(actual, expected);

        let actual = run_tokenizer("q Q W* q Q");
        let expected = vec![
            Token::SaveGraphicsState,
            Token::RestoreGraphicsState,
            Token::WStarKeyword,
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
