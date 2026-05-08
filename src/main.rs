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
use viewer::{PageCtx, view_contents};

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
    let resource_ref = page.get("Resources");
    let resources = match resource_ref {
        pdf::ast::Value::Reference{ .. } => resource_ref.deref(&ast),
        _ => resource_ref,
    };
    let fonts = resources.get("Font");
    let font_lib = ast.process_fonts(fonts);

    let cs = resources.get("ColorSpace");
    let cs_lib = ast.process_colour_spaces(cs);

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
        cs_lib: cs_lib,
    };

    let decoded_contents = contents.decode();
    println!("Content:\n{}", contents.decode_to_string());
    let tokenized_contents = tokenize_stream(decoded_contents);
    view_contents(&page_ctx, &tokenized_contents);
}
