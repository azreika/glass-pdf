use std::fmt;
use std::collections::HashMap;
use flate2::read::ZlibDecoder;
use std::io::prelude::*;
use crate::{fonts::{Font, FontLib}, pdf, viewer::PageCtx};

#[derive(Debug, Copy, Clone)]
pub struct SrcLoc {
    pos: usize,
}

impl SrcLoc {
    pub fn new(pos: usize) -> Self {
        return SrcLoc {
            pos,
        }
    }
}

impl fmt::Display for SrcLoc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.pos)
    }
}

#[derive(Debug, Clone)]
pub struct XObject {
    pub width: u32,
    pub height: u32,
    pub bytes: Vec<u8>,
    pub cs: String,
    pub mask: Option<Box<XObject>>,
}


#[derive(Debug, Clone)]
pub struct XObjectLib {
    pub id_to_obj: HashMap<String,XObject>,
}

impl XObjectLib {
    pub fn new() -> Self {
        return XObjectLib { id_to_obj: HashMap::new(), };
    }

    pub fn insert(&mut self, id: String, xobj: XObject) {
        self.id_to_obj.insert(id, xobj);
    }
}

#[derive(Debug,Clone)]
pub struct ColourSpace {
    pub num_components: u8,
}

#[derive(Debug,Clone)]
pub struct ColourSpaceLib {
    pub id_to_cs: HashMap<String,ColourSpace>,
}

impl ColourSpaceLib {
    pub fn new() -> Self {
        let mut id_to_cs = HashMap::new();
        id_to_cs.insert("DeviceGray".to_string(), ColourSpace { num_components: 1 });
        id_to_cs.insert("DeviceRGB".to_string(), ColourSpace { num_components: 3 });
        return ColourSpaceLib { id_to_cs };
    }

    fn add_cs(&mut self, id: String, cs: ColourSpace) {
        self.id_to_cs.insert(id, cs);
    }

    pub fn num_components(&self, cs: String) -> u8 {
        return self.id_to_cs.get(&cs).unwrap().num_components;
    }
}

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
    Boolean(bool),
    Null,
}

impl Value {
    pub fn to_xobj(&self) -> XObject {
        let metadata = self.metadata();
        let width = metadata.get("Width").unwrap().to_num() as u32;
        let height = metadata.get("Height").unwrap().to_num() as u32;
        let cs = metadata.get("ColorSpace").unwrap().get_string();

        let filter_type = metadata.get("Filter").unwrap().get_string();
        assert_eq!(filter_type, "FlateDecode");

        let subtype = metadata.get("Subtype").unwrap().get_string();
        assert_eq!(subtype, "Image");

        let bytes = self.decode();
        return XObject {
            width,
            height,
            bytes,
            cs,
            mask: None,
        };
    }

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

    pub fn try_get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Dict(map) => {
                return map.get(key);
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

    pub fn decode(&self) -> Vec<u8> {
        let bytes = self.bytes();

        let metadata = self.metadata();
        let filter = metadata.get("Filter").unwrap().get_string();
        assert!(filter == "FlateDecode");

        let mut z = ZlibDecoder::new(&bytes[..]);
        let mut w = Vec::new();
        z.read_to_end(&mut w).unwrap();
        return w;
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

    pub fn to_vec_f32(&self) -> Vec<f32> {
        let arr = match self {
            Value::Vector(arr) => {
                arr.iter().map(|c| return c.to_num()).collect()
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

    pub fn process_colour_spaces(&self, cs: &Value) -> ColourSpaceLib {
        let mut cs_lib = ColourSpaceLib::new();
        for (id, val) in cs.get_dict() {
            let obj = val.deref(&self);
            assert!(matches!(obj, Value::Vector(_)));
            let vv = obj.get_vec();
            assert_eq!(vv.len(), 2);
            let cs_id = vv[0].get_string();
            assert_eq!(cs_id, "ICCBased");
            let stream = vv[1].deref(&self);

            let meta = stream.metadata();
            let n = meta.get("N").unwrap().to_num() as u8;
            assert!(matches!(n, 1 | 3 | 4));

            let cs = ColourSpace {
                num_components: n,
            };
            cs_lib.add_cs(id.to_string(), cs);
        }
        return cs_lib;
    }

    pub fn process_fonts(&self, fonts: &Value) -> FontLib {
        let mut id_to_font = HashMap::new();
        for (id, obj) in fonts.get_dict() {
            let obj_info = obj.deref(&self);
            assert_eq!(obj_info.get("Type").get_string(), "Font");
            let descriptor = obj_info.get("FontDescriptor").deref(&self);
            let widths_ref = obj_info.get("Widths");
            let widths = if Value::is_obj_ref(widths_ref) {
                widths_ref.deref(&self)
            } else {
                widths_ref
            };

            let maybe_enc = obj_info.try_get("Encoding");
            // TODO: handle differente encodings
            let encoding = if let Some(enc) = maybe_enc {
                Some(enc.get_string())
            } else {
                None
            };

            let subtype = obj_info.get("Subtype").get_string();
            assert_eq!(subtype, "TrueType");

            let font_file = descriptor.get("FontFile2").deref(&self);
            let bb = font_file.decode();

            let inner_name = descriptor.get("FontName").get_string();

            let ff = fontdue::Font::from_bytes(bb, fontdue::FontSettings::default()).unwrap();
            let font = Font {
                id: id.to_string(),
                name: inner_name,
                widths: widths.to_vec_u32(),
                first_char: obj_info.get("FirstChar").to_num() as u32,
                ttf: ff,
                encoding,
            };
            id_to_font.insert(id.to_string(), font);

        }
        return FontLib {
            id_to_font,
        };
    }

    pub fn process_xobjs(&self, xobjs: &Value) -> XObjectLib {
        let mut lib = XObjectLib::new();

        assert!(matches!(xobjs, Value::Dict(_)));
        for (id, vref) in xobjs.get_dict() {
            assert!(matches!(vref, Value::Reference { .. }));
            let v = vref.deref(&self);
            assert!(matches!(v, Value::ByteStream(_, _)));
            let metadata = v.metadata();
            println!("XOBJ: {id} {:?}", metadata);

            let mut xobj = v.to_xobj();
            assert_eq!(xobj.cs, "DeviceRGB");

            let smask = metadata.get("SMask").unwrap().deref(&self).to_xobj();
            assert_eq!(smask.cs, "DeviceGray");
            xobj.mask = Some(Box::new(smask));

            lib.insert(id.to_string(), xobj);
        }

        return lib;
    }

    pub fn mk_page_ctx(&self, page: &Value) -> PageCtx {
        let resource_ref = page.get("Resources");
        let resources = match resource_ref {
            pdf::ast::Value::Reference{ .. } => resource_ref.deref(&self),
            _ => resource_ref,
        };

        let fonts = resources.get("Font");
        let font_lib = self.process_fonts(fonts);

        let maybe_cs = resources.try_get("ColorSpace");
        let cs_lib = if let Some(cs) = maybe_cs {
            self.process_colour_spaces(cs)
        } else {
            ColourSpaceLib { id_to_cs: HashMap::new() }
        };

        let maybe_xobjs = resources.try_get("XObject");
        let xobj_lib = if let Some(xobjs) = maybe_xobjs {
            self.process_xobjs(xobjs)
        } else {
            XObjectLib::new()
        };

        let media_box = page.get("MediaBox").to_vec_f32();
        assert_eq!(media_box.len(), 4);
        assert_eq!(media_box[0], 0.0);
        assert_eq!(media_box[1], 0.0);
        let page_width = media_box[2] as f64;
        let page_height = media_box[3] as f64;

        return PageCtx {
            height: page_height,
            width: page_width,
            font_lib: font_lib,
            cs_lib: cs_lib,
            xobj_lib: xobj_lib,
        };
    }
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
            },
            Value::Boolean(x) => {
                write!(f, "{x}")
            },
            Value::Null => {
                write!(f, "NULL")
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
