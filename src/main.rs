use std::fs;

mod tokenizer;
mod parser;
mod ast;
mod content;

use crate::tokenizer::{tokenize_pdf};
use crate::parser::{parse_tokens};
use crate::content::{parse_stream};

fn main() {
    let data: Vec<u8> = fs::read("./src/samplepdf.pdf").expect("woops");
    let tokens = tokenize_pdf(&data);
    let ast = parse_tokens(&tokens);
    println!("{}", ast);

    println!("-----------");

    let trailer = ast.get_trailer_dict();
    let root_ref = trailer.get("Root").unwrap();
    println!("Root: {root_ref}");
    let root = ast.get_object(root_ref);
    println!("{root_ref}: {root}");
    let pages_ref = root.get("Pages");
    let pages = ast.get_object(pages_ref);

    let kids = pages.get("Kids");
    let vec = kids.get_vec();
    assert_eq!(vec.len(), 1);
    let page = ast.get_object(&vec[0]);
    println!("Page: {}", page);
    let contents = page.get("Contents").deref(&ast);
    let result = contents.decode();
    parse_stream(result);
}
