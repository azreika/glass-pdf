use std::fs;

mod tokenizer;
mod pdf;
mod viewer;
mod content;
mod viewer_message;
mod fonts;
mod test_consts;
mod transform;

use pdf::tokenizer::{tokenize_pdf};
use pdf::parser::{parse_tokens};
use content::tokenizer::{tokenize_stream};
use viewer::view_contents;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let fpath = if args.len() < 2 {
        "./examples/samplepdf.pdf".to_string()
    } else {
        assert_eq!(args.len(), 2);
        args[1].clone()
    };

    let data: Vec<u8> = fs::read(fpath).expect("woops");
    let tokens = tokenize_pdf(&data);
    let ast = parse_tokens(&tokens);
    println!("{}", ast);

    println!("-----------");

    let trailer = ast.get_trailer_dict();
    let root_ref = trailer.get("Root").unwrap();
    let root = ast.get_object(root_ref);
    let pages_ref = root.get("Pages");
    let pages = ast.get_object(pages_ref);
    let kids = pages.get("Kids");
    let vec = kids.get_vec();
    assert!(vec.len() >= 1);
    let page = ast.get_object(&vec[0]);
    let contents = page.get("Contents").deref(&ast);

    let ctx = ast.mk_page_ctx(page);
    let decoded_contents = contents.decode();
    println!("Content:\n{}", contents.decode_to_string());
    let tokenized_contents = tokenize_stream(decoded_contents);
    view_contents(&ctx, &tokenized_contents);
}
