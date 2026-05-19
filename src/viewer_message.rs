use crate::{content::graphics::{ClippingRule, PathOp}, pdf::ast::XObject};

#[derive(Clone, Debug)]
pub enum Message {
    DrawBlock(Vec<Message>),
    DrawGlyph(GlyphInfo),
    DrawPath(PathInfo),
    DrawXObject(XObjectInfo),
    Noop,
}

impl Message {
    pub fn is_noop(&self) -> bool {
        return matches!(self, Message::Noop);
    }
}

#[derive(Clone, Debug)]
pub struct GlyphInfo {
    pub x: f64,
    pub y: f64,
    pub byte: u8,
    pub size: f64,
    pub font_id: String,
    pub width: f64,
    pub colour: Color,
    pub clips: Vec<(ClippingRule,Vec<PathOp>)>,
}

#[derive(Clone, Debug)]
pub struct PathInfo {
    pub path: Vec<PathOp>,
    pub colour: Color,
    pub rule: ClippingRule,
    pub clips: Vec<(ClippingRule,Vec<PathOp>)>,
}

#[derive(Clone, Debug)]
pub struct XObjectInfo {
    pub bytes: Vec<u8>,
    pub x: f64,
    pub y: f64,
    pub w: u32,
    pub h: u32,
    pub x_scale: f64,
    pub y_scale: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    RGB(f32, f32, f32),
    RGBA(f32, f32, f32, f32),
    Gray(f32),
    Default,
}

impl Color {
    pub fn to_rgb8(&self) -> [u8; 3] {
        let to_u8 = |t| (t * 255.0) as u8;
        match self {
            Color::RGB(r, g, b) => [to_u8(r), to_u8(g), to_u8(b)],
            Color::Gray(g) => [to_u8(g), to_u8(g), to_u8(g)],
            _ => panic!("unexpected color"),
        }

    }
}
