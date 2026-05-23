use std::collections::HashMap;
use std::fs;
use std::io::stdout;

mod tokenizer;
mod pdf;
mod viewer;
mod content;
mod viewer_message;
mod fonts;
mod test_consts;
mod transform;
mod view_info;

use content::tokenizer::{tokenize_stream};
use viewer::view_contents;

use std::env;

use crate::content::pretty::PrettyPrinter;
use crate::fonts::FontLib;
use crate::pdf::ast::{ColourSpaceLib, GStateLib, Pdf, Value, XObjectLib};
use crate::pdf::parse_pdf;
use crate::viewer::PageCtx;

fn get_pages(ast: &Pdf) -> &Vec<Value> {
    let trailer = ast.get_trailer_dict();
    let root_ref = trailer.get("Root").unwrap();
    let root = ast.get_object(root_ref);
    let pages_ref = root.get("Pages");
    let pages = ast.get_object(pages_ref);
    let kids = pages.get("Kids");
    return kids.get_vec();
}

enum RunType {
    PDF,
    ContentStream,
}

struct RunConfig {
    filename: String,
    run_type: RunType,
}

fn read_args() -> RunConfig {
    let mut args = env::args();

    // skip program name
    args.next();

    let mut is_content = false;
    let mut filename = None;
    for arg in args {
        match arg.as_str() {
            "--content" => is_content = true,
            other => {
                assert!(!other.starts_with("-"));
                assert!(filename.is_none());
                filename = Some(other.to_string());
            },
        }
    }
    let filename = filename.unwrap_or("examples/samplepdf.pdf".to_string());

    let run_type = if is_content {
        RunType::ContentStream
    } else {
        RunType::PDF
    };

    return RunConfig {
        filename,
        run_type,
    };
}

fn view_pdf(filename: String) {
    let pdf_bytes = fs::read(filename).expect("woops");

    let ast = parse_pdf(&pdf_bytes);
    println!("{}", ast);
    println!("-----------");

    let pages = get_pages(&ast);
    assert!(pages.len() >= 1);
    let page = ast.get_object(&pages[0]);

    let contents = page.get("Contents").deref(&ast);
    let ctx = ast.mk_page_ctx(page);

    let decoded_contents = contents.decode();
    let tokenized_contents = tokenize_stream(decoded_contents);
    PrettyPrinter::pretty_print(&mut stdout(), &tokenized_contents);

    view_contents(&ctx, &tokenized_contents);
}

fn view_content_stream(filename: String) {
    let content_bytes = fs::read(filename).expect("woops");
    let ctx = PageCtx {
        width: 500.0,
        height: 500.0,
        font_lib: FontLib { id_to_font: HashMap::new() },
        cs_lib: ColourSpaceLib { id_to_cs: HashMap::new() },
        xobj_lib: XObjectLib::new(),
        gstate_lib: GStateLib::new(),
    };
    let tokenized_contents = tokenize_stream(content_bytes);
    view_contents(&ctx, &tokenized_contents);
}

fn main() {
    let config = read_args();

    match config.run_type {
        RunType::PDF => {
            view_pdf(config.filename);
        },
        RunType::ContentStream => {
            view_content_stream(config.filename);
        }
    };
}
