use std::fs;

mod tokenizer;
mod parser;
mod ast;

use crate::tokenizer::{tokenize_pdf};
use crate::parser::{parse_tokens};

fn main() {
    let data: Vec<u8> = fs::read("samplepdf.pdf").expect("woops");
    let tokens = tokenize_pdf(&data);
    for token in tokens.iter() {
        print!("{} ", token);
    }
    println!("");

    let ast = parse_tokens(&tokens);
    println!("{:?}", ast);
}
