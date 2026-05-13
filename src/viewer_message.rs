use crate::content::streamer::{ClippingRule, PathPiece};

#[derive(Clone, Debug)]
pub enum Message {
    DrawBlock(Vec<Message>),
    DrawGlyph(GlyphInfo),
    DrawPath(PathInfo),
    Clip(PathInfo),
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
    pub colour: Option<Vec<f64>>,
}

#[derive(Clone, Debug)]
pub struct PathInfo {
    pub path: Vec<PathPiece>,
    pub colour: Option<Vec<f64>>,
    pub rule: ClippingRule,
}
