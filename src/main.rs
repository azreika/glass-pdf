use std::fs;

mod tokenizer;
mod parser;
mod ast;
mod viewer;
mod pdf_tokenizer;
mod content_tokenizer;
mod src_loc;

use crate::pdf_tokenizer::{tokenize_pdf};
use crate::parser::{parse_tokens};
use crate::content_tokenizer::{tokenize_stream};
use crate::viewer::{view_contents};

fn main() {
    let data: Vec<u8> = fs::read("./examples/samplepdf.pdf").expect("woops");
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

    let resources = page.get("Resources").deref(&ast);
    println!("Resources:\n{}", resources);
    let fonts = resources.get("Font");
    let font_lib = ast.process_fonts(fonts);

    let decoded_contents = contents.decode();

    println!("Content:\n{}", decoded_contents);
    let tokenized_contents = tokenize_stream(decoded_contents);
    view_contents(&font_lib, &tokenized_contents);
}
