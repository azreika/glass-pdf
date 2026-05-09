use crate::tokenizer::Tokenizer;

#[derive(Clone, Debug, PartialEq)]
pub enum ContentToken {
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

impl Tokenizer<ContentToken> for ContentTokenizer {
    fn token_from_word(&self, word: &str) -> ContentToken {
        return match word {
            "SC" => ContentToken::SetColourStroke,
            "sc" => ContentToken::SetColourNoStroke,
            "f" => ContentToken::Fill,
            "i" => ContentToken::IKeyword,
            "cs" => ContentToken::CsNoStroke,
            "CS" => ContentToken::CsStroke,
            "cm" => ContentToken::CmStroke,
            "BT" => ContentToken::BTKeyword,
            "Tm" => ContentToken::TmKeyword,
            "Tf" => ContentToken::TfKeyword,
            "Tj" => ContentToken::TjKeyword,
            "ET" => ContentToken::ETKeyword,
            "[" => ContentToken::LBracket,
            "]" => ContentToken::RBracket,
            "TJ" => ContentToken::TJKeyword,
            "re" => ContentToken::RectKeyword,
            "m" => ContentToken::MKeyword,
            "l" => ContentToken::LKeyword,
            "h" => ContentToken::HKeyword,
            "v" => ContentToken::VKeyword,
            "c" => ContentToken::CKeyword,
            "y" => ContentToken::YKeyword,
            "<" => ContentToken::AngleOpen,
            ">" => ContentToken::AngleClose,
            "BDC" => ContentToken::BMCKeyword,
            "EMC" => ContentToken::EMCKeyword,
            "BMC" => ContentToken::BMCKeyword,
            "gs" => ContentToken::GSKeyword,
            "g" => ContentToken::GNonStroke,
            "G" => ContentToken::GStroke,
            "rg" => ContentToken::RGNonStroke,
            "RG" => ContentToken::RGStroke,
            "*" => ContentToken::Star,
            "w" => ContentToken::WLineWidth,
            "J" => ContentToken::LineCap,
            "j" => ContentToken::LineJoin,
            "S" => ContentToken::Stroke,
            "Tc" => ContentToken::CharSpacing,
            "Do" => ContentToken::DoKeyword,
            "Q" => ContentToken::RestoreGraphicsState,
            "q" => ContentToken::SaveGraphicsState,
            "n" => ContentToken::NKeyword,
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

    fn lex_w(&mut self) -> ContentToken {
        self.eat_char('W');
        if self.peek() == '*' {
            self.eat_char('*');
            return ContentToken::WStarKeyword;
        } else {
            return ContentToken::WKeyword;
        }
    }

    fn lex_identifier(&mut self) -> ContentToken {
        let id = self.lex_word();
        return ContentToken::Identifier(id);
    }

    fn lex_string(&mut self) -> ContentToken {
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
        return ContentToken::StringBytes(bytes);
    }

    fn lex_keyword(&mut self) -> ContentToken {
        let word = self.lex_word();
        return self.token_from_word(&word);
    }

    fn lex_number(&mut self) -> ContentToken {
        let number = self.parse_next_number();
        return ContentToken::Number(number);
    }

    fn run(&mut self) -> Vec<ContentToken>{
        self.offset = 0;
        let mut toks = vec![];

        while self.offset < self.data.len() {
            match self.peek() {
                'W' => { toks.push(self.lex_w())},
                '/' => { self.lex_char(); toks.push(self.lex_identifier()); },
                '(' => { self.lex_char(); toks.push(self.lex_string()); },
                '\0' => { self.lex_char(); toks.push(ContentToken::Null); }
                c if c.is_whitespace() => { self.lex_char(); },
                c if c.is_numeric() || c == '-' => { toks.push(self.lex_number()); },
                _ => { toks.push(self.lex_keyword()); }
            };
        }
        return toks;
    }
}

pub fn tokenize_stream(str: Vec<u8>) -> Vec<ContentToken> {
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

    fn run_tokenizer(str: &str) -> Vec<ContentToken> {
        return tokenize_stream(str_bytes(str));
    }

    #[test]
    fn simple_seqs() {
        let actual = run_tokenizer("
        q Q q Q q Q
        ");
        let expected = vec![
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn escaped_strings() {
        let actual = run_tokenizer("
        (\\(hello\\))
        ");
        let expected = vec![
            ContentToken::StringBytes(str_bytes("\\(hello\\)")),
        ];
        assert_eq!(actual, expected);

        let actual = run_tokenizer("
        (hello\\n)
        ");
        let expected = vec![
            ContentToken::StringBytes(str_bytes("hello\\n")),
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn w_keywords() {
        let actual = run_tokenizer("q Q W q Q");
        let expected = vec![
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
            ContentToken::WKeyword,
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
        ];
        assert_eq!(actual, expected);

        let actual = run_tokenizer("q Q W* q Q");
        let expected = vec![
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
            ContentToken::WStarKeyword,
            ContentToken::SaveGraphicsState,
            ContentToken::RestoreGraphicsState,
        ];
        assert_eq!(actual, expected);
    }

    #[test]
    fn sample_pdf() {
        // Just check that it doesn't crash
        run_tokenizer(SAMPLE_PDF_STREAM);
    }

}
