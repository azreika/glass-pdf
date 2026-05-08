use crate::pdf::{ast::Pdf, parser::parse_tokens, tokenizer::tokenize_pdf};

pub mod parser;
pub mod tokenizer;
pub mod ast;

pub fn parse_pdf(bytes: &Vec<u8>) -> Pdf {
    let tokens = tokenize_pdf(&bytes);
    return parse_tokens(&tokens);
}
