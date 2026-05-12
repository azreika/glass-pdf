use std::{io::Write};

use crate::content::tokenizer::Token;

pub struct PrettyPrinter<'a> {
    indent: u32,
    offset: usize,
    tokens: &'a Vec<Token>,
}

impl PrettyPrinter<'_> {
    fn write<T>(&mut self, io: &mut T, str: &str) where T: Write {
        self.write_raw(io, str);
    }
    fn w_newline<T>(&mut self, io: &mut T) where T: Write {
        self.write_raw(io, "\n");
        self.w_indent(io);
    }
    fn w_indent<T>(&mut self, io: &mut T) where T: Write {
        for _ in 0..self.indent {
            self.write_raw(io, " ");
        }
    }

    fn write_raw<T>(&mut self, io: &mut T, str: &str) where T: Write {
        io.write(str.as_bytes()).unwrap();
    }

    pub fn pretty_print<T>(io: &mut T, toks: &Vec<Token>) where T: Write {
        let mut printer = PrettyPrinter {
            indent: 0,
            offset: 0,
            tokens: toks,
        };

        printer.write_all(io);
    }

    fn next_token(&mut self) -> &Token {
        let tok = &self.tokens[self.offset];
        self.offset += 1;
        return tok;
    }

    fn has_next(&self) -> bool {
        return self.offset < self.tokens.len();
    }

    fn indent_in(&mut self) {
        self.indent += 4;
    }
    fn indent_out(&mut self) {
        self.indent -= 4;
    }

    fn write_all<T>(&mut self, io: &mut T) where T: Write {
        self.write(io, "<CONTENTS>");
        self.indent_in();
        self.w_newline(io);

        while self.has_next() {
            let tok = self.next_token();
            match tok {
                Token::BT | Token::BDC | Token::BMC => {
                    let str = to_txt(tok.clone());
                    self.w_newline(io);
                    self.write(io, &str);
                    self.indent_in();
                    self.w_newline(io);
                },
                Token::ET | Token::EMC => {
                    let str = to_txt(tok.clone());
                    self.indent_out();
                    self.w_newline(io);
                    self.write(io, &str);
                    self.w_newline(io);
                },
                tt => {
                    let txt = to_txt(tt.clone());
                    self.write(io, &txt);
                    self.write(io, " ");
                },
            }
        };

        self.indent_out();
        self.w_newline(io);
        self.write(io, "</CONTENTS>");
        self.w_newline(io);
    }
}

fn to_txt(tok: Token) -> String {
    match tok {
        Token::Number(f) => return f.to_string(),
        Token::Identifier(str) => return "/".to_string() + &str.to_string(),
        Token::Array(vv) => {
            return vv.iter().map(|t| to_txt(t.clone())).collect::<String>();
        },
        Token::Dict(vv) => {
            return "{".to_string() + &vv.iter().map(|(k, v)|
                k.clone() + " => " + &to_txt(v.clone())
            ).collect::<String>() + "}";
        },
        Token::StringBytes(vv) => {
            return "(\"".to_string() + &vv.iter().map(|cc|
                (*cc as char).to_string()
            ).collect::<String>() + "\")";
        }
        _ => {},
    }

    return match tok {
        Token::SetColourStroke => "SC",
        Token::SetColourNoStroke => "sc",
        Token::Fill => "f",
        Token::I => "i",
        Token::CsNoStroke => "cs",
        Token::CsStroke => "CS",
        Token::CmStroke => "cm",
        Token::Tm => "Tm",
        Token::Tf => "Tf",
        Token::Tj => "Tj",
        Token::TJ => "TJ",
        Token::Rect => "re",
        Token::M => "m",
        Token::L => "l",
        Token::H => "h",
        Token::V => "v",
        Token::C => "c",
        Token::Y => "y",
        Token::BDC => "BDC",
        Token::EMC => "EMC",
        Token::BMC => "BMC",
        Token::BT => "BT",
        Token::ET => "ET",
        Token::GS => "gs",
        Token::GNonStroke => "g",
        Token::GStroke => "G",
        Token::RGNonStroke => "rg",
        Token::RGStroke => "RG",
        Token::Star => "*",
        Token::WLineWidth => "w",
        Token::LineCap => "J",
        Token::LineJoin => "j",
        Token::Stroke => "S",
        Token::CharSpacing => "Tc",
        Token::Do => "Do",
        Token::RestoreGraphicsState => "Q",
        Token::SaveGraphicsState => "q",
        Token::N => "n",
        Token::W => "W",
        Token::WStar => "W*",
        _ => "unknown",
    }.to_string();
}
