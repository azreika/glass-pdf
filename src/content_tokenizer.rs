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
}

impl ContentToken {
    pub fn value(&self) -> f64 {
        return match self {
            ContentToken::Number(v) => *v,
            _ => panic!(),
        }
    }

    pub fn ident(&self) -> String {
        return match self {
            ContentToken::Identifier(v) => v.clone(),
            _ => panic!(),
        }
    }
}

struct ContentTokenizer {
    data: Vec<u8>,
    offset: usize,
    missed_words: u32,
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

pub fn tokenize_stream(str: String) -> Vec<ContentToken> {
    let mut vv = vec![];

    let bytes_arr = str.as_bytes().into_iter().copied().collect();
    let mut tokenizer = ContentTokenizer { data: bytes_arr, offset: 0, missed_words: 0 };
    while tokenizer.offset < tokenizer.data.len() {
        let cc = tokenizer.peek();
        if cc.is_whitespace() {
            tokenizer.lex_char();
        } else if cc == '\0' {
            tokenizer.lex_char();
            vv.push(ContentToken::Null);
        } else if cc == 'q' {
            tokenizer.lex_char();
            vv.push(ContentToken::SaveGraphicsState);
        } else if cc == 'Q' {
            tokenizer.lex_char();
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
            let id = tokenizer.lex_word();
            let id_tok = ContentToken::Identifier(id);
            vv.push(id_tok);
        } else {
            let word = tokenizer.lex_word();
            let tok = tokenizer.token_from_word(&word);
            vv.push(tok.clone());

            if matches!(tok, ContentToken::LParens) {
                let mut chars = vec![];
                let mut depth = 1;
                while depth > 0 && tokenizer.offset < tokenizer.data.len() {
                    let mm = tokenizer.lex_char();
                    match mm {
                        '\\' => {
                            chars.push(mm);
                            if tokenizer.offset < tokenizer.data.len() {
                                chars.push(tokenizer.lex_char()); // consume \(, \), \\, \n, etc.
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
