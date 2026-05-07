use std::fs;

mod tokenizer;
mod parser;
mod ast;
mod viewer;
mod pdf_tokenizer;
mod content_tokenizer;
mod content_streamer;
mod viewer_message;
mod src_loc;
mod fonts;
mod transform;

use crate::pdf_tokenizer::{tokenize_pdf};
use crate::parser::{parse_tokens};
use crate::content_tokenizer::{tokenize_stream};
use crate::viewer::{PageCtx, view_contents};

fn main() {
    let data: Vec<u8> = fs::read("./examples/NDIS_pricing.pdf").expect("woops");
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
    assert!(vec.len() >= 1);
    let page = ast.get_object(&vec[0]);
    println!("Page: {}", page);

    let contents = page.get("Contents").deref(&ast);

    let resource_ref = page.get("Resources");
    let resources = match resource_ref {
        ast::Value::Reference{ .. } => resource_ref.deref(&ast),
        _ => resource_ref,
    };
    println!("Resources:\n{}", resources);
    let fonts = resources.get("Font");
    let font_lib = ast.process_fonts(fonts);

    let media_box = page.get("MediaBox").to_vec_f32();
    assert_eq!(media_box.len(), 4);
    assert_eq!(media_box[0], 0.0);
    assert_eq!(media_box[1], 0.0);
    let page_width = media_box[2] as f64;
    let page_height = media_box[3] as f64;
    let page_ctx = PageCtx {
        height: page_height,
        width: page_width,
        font_lib: font_lib,
        scale_factor: 1.0,
    };

    let decoded_contents = contents.decode();
    println!("Content:\n{}", contents.decode_to_string());
    let tokenized_contents = tokenize_stream(decoded_contents);
    view_contents(&page_ctx, &tokenized_contents);
}
