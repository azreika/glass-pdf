use crate::content::streamer::PathPiece;

#[derive(Clone, Debug)]
pub enum Message {
    DrawBlock(Vec<Message>),
    DrawGlyph(GlyphInfo),
    SetScaleFactor(f32),
    DrawPath(PathInfo),
    Noop,
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

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum State {
    TopLevel,
    Text,
    MarkedContent,
}

#[derive(Clone, Debug)]
pub struct PathInfo {
    pub path: Vec<PathPiece>,
    pub colour: Option<Vec<f64>>,
}
