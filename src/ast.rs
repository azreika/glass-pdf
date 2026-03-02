
use std::fmt;
use crate::tokenizer::SrcLoc;

#[derive(Debug)]
pub enum Block {
    Object {
        id: i32,
        gxn: i32,
        body: Value,
        loc: SrcLoc,
    },
    XRefTable(Vec<SrcLoc>, SrcLoc),
}

#[derive(Debug)]
pub enum Value {
    ByteStream(Vec<u8>),
    Number(i32),
}

#[derive(Debug)]
pub struct Pdf {
    pub blocks: Vec<Block>,
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Block::Object { id, gxn, body, loc } => {
                write!(f, "loc.{loc}: object {id} {gxn}\n").unwrap();
                match body {
                    Value::ByteStream(vec) => write!(f, "\t<{} bytes...>", vec.len()),
                    Value::Number(v) => write!(f, "\t{}", v),
                }
            },
            Block::XRefTable(offsets, loc) => {
                write!(f, "loc.{loc}: xref\n").unwrap();
                for (i, offset) in offsets.iter().enumerate() {
                    write!(f, "\t{i} => {offset}\n").unwrap();
                }
                Ok(())
            },
        }
    }
}

impl fmt::Display for Pdf {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for object in self.blocks.iter() {
            write!(f, "{}\n", object).unwrap();
        }
        write!(f, "%%EOF")
    }
}
