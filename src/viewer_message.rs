#[derive(Clone, Debug)]
pub enum Message {
    DrawBlock(Vec<Message>),
    DrawGlyph(GlyphInfo),
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
}

#[derive(Copy, Clone)]
pub enum State {
    TopLevel,
    InText,
}
