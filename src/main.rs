use std::fs;

mod tokenizer;
mod pdf;
mod viewer;
mod content;
mod viewer_message;
mod fonts;
mod test_consts;
mod transform;

use content::tokenizer::{tokenize_stream};
use viewer::view_contents;

use std::env;

use crate::pdf::ast::{Pdf, Value};
use crate::pdf::parse_pdf;

fn read_pdf_bytes() -> Vec<u8> {
    let args: Vec<String> = env::args().collect();
    let fpath = if args.len() < 2 {
        "./examples/samplepdf.pdf".to_string()
    } else {
        assert_eq!(args.len(), 2);
        args[1].clone()
    };
    return fs::read(fpath).expect("woops");
}

fn get_pages(ast: &Pdf) -> &Vec<Value> {
    let trailer = ast.get_trailer_dict();
    let root_ref = trailer.get("Root").unwrap();
    let root = ast.get_object(root_ref);
    let pages_ref = root.get("Pages");
    let pages = ast.get_object(pages_ref);
    let kids = pages.get("Kids");
    return kids.get_vec();
}

fn main() {
    let pdf_bytes = read_pdf_bytes();
    let ast = parse_pdf(&pdf_bytes);
    println!("{}", ast);
    println!("-----------");

    let pages = get_pages(&ast);
    assert!(pages.len() == 1);
    let page = ast.get_object(&pages[0]);
    let contents = page.get("Contents").deref(&ast);
    let ctx = ast.mk_page_ctx(page);

    let decoded_contents = contents.decode();
    println!("Content:\n{}", contents.decode_to_string());
    let tokenized_contents = tokenize_stream(decoded_contents);
    view_contents(&ctx, &tokenized_contents);
}
