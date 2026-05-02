
use std::fmt;
use crate::src_loc::SrcLoc;
use std::collections::HashMap;
use flate2::read::ZlibDecoder;
use std::io::prelude::*;

#[derive(Debug)]
pub enum Block {
    Object {
        id: i32,
        gxn: i32,
        body: Value,
        loc: SrcLoc,
    },
    XRefTable(Vec<SrcLoc>, SrcLoc),
    Trailer(HashMap<String,Value>),
}

#[derive(Debug)]
pub enum Value {
    ByteStream(Box<Value>, Vec<u8>),
    Number(f32),
    Reference { id: i32, gxn: i32 },
    Dict(HashMap<String,Value>),
    Vector(Vec<Value>),
    Identifier(String),
}

impl Value {
    pub fn is_obj_ref(v: &Value) -> bool {
        return match v {
            Value::Reference{id:_,gxn:_} => true,
            _ => false,
        }
    }

    pub fn from_dict(dict: HashMap<String, Value>) -> Value {
        return Value::Dict(dict);
    }

    pub fn obj_matches(obj: &Value, t_id: i32, t_gxn: i32) -> bool {
        match obj {
            Value::Reference{id, gxn} => {
                if *id == t_id && *gxn == t_gxn {
                    return true;
                }
            }
            _ => {()},
        }
        return false;
    }

    pub fn get(&self, key: &str) -> &Value {
        match self {
            Value::Dict(map) => {
                return map.get(key).unwrap();
            }
            _ => panic!("expected dict, got {}", key),
        }
    }

    pub fn get_vec(&self) -> &Vec<Value> {
        match self {
            Value::Vector(vec) => return vec,
            _ => panic!(),
        }
    }

    pub fn deref<'a>(&self, pdf: &'a Pdf) -> &'a Value {
        return pdf.get_object(&self);
    }

    #[allow(dead_code)]
    pub fn show(&self) {
        println!("{:?}", self);
    }

    pub fn bytes(&self) -> &Vec<u8> {
        match self {
            Value::ByteStream(_, bytes) => &bytes,
            _ => panic!(),
        }
    }

    pub fn get_dict(&self) -> &HashMap<String,Value> {
        match self {
            Value::Dict(map) => map,
            _ => panic!(),
        }
    }

    pub fn get_string(&self) -> String {
        match self {
            Value::Identifier(str) => str.to_string(),
            _ => panic!(),
        }
    }

    pub fn metadata(&self) -> &HashMap<String,Value> {
        match self {
            Value::ByteStream(metadata, _) => metadata.get_dict(),
            _ => panic!(),
        }
    }

    pub fn decode(&self) -> String {
        let bytes = self.bytes();

        let metadata = self.metadata();
        let filter = metadata.get("Filter").unwrap().get_string();
        assert!(filter == "FlateDecode");

        let mut z = ZlibDecoder::new(&bytes[..]);
        let mut w = Vec::new();
        z.read_to_end(&mut w).unwrap();

        let mut str = String::new();
        for &b in w.iter() {
            str += &(b as char).to_string();
        }
        return str;
    }

    fn to_vec_u32(&self) -> Vec<u32> {
        let arr = match self {
            Value::Vector(arr) => {
                arr.iter().map(|c| return c.to_num() as u32).collect()
            }
            _ => panic!(),
        };
        return arr;
    }
}

#[derive(Debug)]
pub struct Pdf {
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug)]
pub struct FontLib {
    id_to_font: HashMap<String, Font>,
}

impl Pdf {
    pub fn get_trailer_dict(&self) -> &HashMap<String,Value> {
        for block in self.blocks.iter() {
            match block {
                Block::Trailer(dict) => return dict,
                _ => {()},
            }
        }
        panic!()
    }

    pub fn get_object(&self, obj: &Value) -> &Value {
        assert!(Value::is_obj_ref(obj));
        for block in self.blocks.iter() {
            match block {
                Block::Object { id, gxn, body, loc:_ } => {
                    if Value::obj_matches(obj, *id, *gxn) {
                        return body;
                    }
                },
                _ => {()},
            }
        }
        panic!();
    }

    pub fn process_fonts(&self, fonts: &Value) -> FontLib {
        let mut id_to_font = HashMap::new();
        for (id, obj) in fonts.get_dict() {
            let obj_info = obj.deref(&self);
            assert_eq!(obj_info.get("Type").get_string(), "Font");
            let descriptor = obj_info.get("FontDescriptor").deref(&self);
            let widths = obj_info.get("Widths").deref(&self);

            let font = Font {
                id: id.to_string(),
                name: descriptor.get("FontName").to_string(),
                widths: widths.to_vec_u32(),
                first_char: obj_info.get("FirstChar").to_num() as u32,
            };
            id_to_font.insert(id.to_string(), font);
        }
        return FontLib {
            id_to_font,
        };
    }
}

#[derive(Clone, Debug)]
struct Font {
    id: String,
    name: String,
    widths: Vec<u32>,
    first_char: u32,
}

impl Value {
    fn to_num(&self) -> f32 {
        return match self {
            Value::Number(x) => *x,
            _ => panic!(),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::ByteStream(metadata, bytes) => {
                write!(f, "{} bytes -- metadata: {}", bytes.len(), metadata)
            },
            Value::Number(v) => {
                write!(f, "{v}")
            },
            Value::Reference { id, gxn } => {
                write!(f, "obj<{id},{gxn}>")
            },
            Value::Dict(dict) => {
                write!(f, "{{\n").unwrap();
                for (k, v) in dict.iter() {
                    write!(f, "\t\t{k} => {v},\n").unwrap();
                }
                write!(f, "\t}}")
            },
            Value::Vector(vec) => {
                write!(f, "[").unwrap();
                for v in vec.iter() {
                    write!(f, "{v},").unwrap();
                }
                write!(f, "]")
            },
            Value::Identifier(id) => {
                write!(f, "\"{id}\"")
            }
        }
    }
}

impl fmt::Display for Block {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Block::Object { id, gxn, body, loc } => {
                write!(f, "loc.{loc}: obj<{id},{gxn}>\n\t{body}")
            },
            Block::XRefTable(offsets, loc) => {
                write!(f, "loc.{loc}: xref\n").unwrap();
                for (i, offset) in offsets.iter().enumerate() {
                    write!(f, "\tobj<{i},0> => loc.{offset}\n").unwrap();
                }
                Ok(())
            },
            Block::Trailer(dict) => {
                write!(f, "Trailer dictionary: {{\n").unwrap();
                for (k, v) in dict.iter() {
                    write!(f, "\t{k} => {v}\n").unwrap();
                }
                write!(f, "}}")
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
