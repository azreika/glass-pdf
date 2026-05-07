use crate::tokenizer::Tokenizer;

#[derive(Clone, Debug)]
pub enum ContentToken {
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
    StringBytes(Vec<u8>),
}

struct ContentTokenizer {
    data: Vec<u8>,
    offset: usize,
    tokens: Vec<ContentToken>,
}

impl Tokenizer<ContentToken> for ContentTokenizer {
    fn token_from_word(&self, word: &str) -> ContentToken {
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
            }
        }
    }

    fn peek_u8(&self) -> u8 {
        return self.data[self.offset];
    }

    fn step_ahead(&mut self) {
        self.offset += 1;
    }
}

impl ContentTokenizer {
    fn new(data: Vec<u8>) -> Self {
        return ContentTokenizer { data, offset: 0, tokens: vec![] };
    }

    fn push_token(&mut self, tok: ContentToken) {
        self.tokens.push(tok);
    }

    fn run(&mut self) {
        while self.offset < self.data.len() {
            let cc = self.peek();
            if cc.is_whitespace() {
                self.lex_char();
            } else if cc == '\0' {
                self.lex_char();
                self.push_token(ContentToken::Null);
            } else if cc == 'q' {
                self.lex_char();
                self.push_token(ContentToken::SaveGraphicsState);
            } else if cc == 'Q' {
                self.lex_char();
                self.push_token(ContentToken::RestoreGraphicsState);
            } else if cc.is_numeric() || cc == '.' || cc == '-' {
                let num = self.lex_number();
                self.push_token(ContentToken::Number(num));
            } else if cc == 'W' {
                self.eat_char('W');
                if self.peek() == '*' {
                    self.eat_char('*');
                    self.push_token(ContentToken::WStarKeyword);
                } else {
                    self.push_token(ContentToken::WKeyword);
                }
            } else if cc == 'n' {
                self.eat_char('n');
                self.push_token(ContentToken::NKeyword);
            } else if cc == '/' {
                self.eat_char('/');
                let id = self.lex_word();
                let id_tok = ContentToken::Identifier(id);
                self.push_token(id_tok);
            } else {
                let word = self.lex_word();
                let tok = self.token_from_word(&word);
                self.push_token(tok.clone());

                if matches!(tok, ContentToken::LParens) {
                    let mut bytes = vec![];
                    let mut depth = 1;
                    while depth > 0 && self.offset < self.data.len() {
                        let mm = self.lex_u8();
                        match mm as char {
                            '\\' => {
                                bytes.push(mm);
                                if self.offset < self.data.len() {
                                    bytes.push(self.lex_u8()); // consume \(, \), \\, \n, etc.
                                }
                            },
                            ')' => {
                                depth -= 1;
                                if depth > 0 { bytes.push(mm); }
                            },
                            '(' => { depth += 1; bytes.push(mm); },
                            _   => bytes.push(mm),
                        }
                    }
                    self.push_token(ContentToken::StringBytes(bytes));
                    self.push_token(ContentToken::RParens);
                }
            }
        }
    }
}

pub fn tokenize_stream(str: String) -> Vec<ContentToken> {
    let bytes = str.as_bytes().into_iter().copied().collect();
    let mut tokenizer = ContentTokenizer::new(bytes);
    tokenizer.run();
    return tokenizer.tokens;
}
