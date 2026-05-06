#[derive(Clone, Debug)]
pub enum Message {
    DrawText { x_pos: i32, y_pos: i32, str: String, size: f32 },
    DrawBlock(Vec<Message>),
    DrawGlyph(GlyphInfo),
    Noop,
}

#[derive(Clone, Debug)]
pub struct GlyphInfo {
    pub x: i32,
    pub y: i32,
    pub str: String,
    pub size: f32,
}

#[derive(Copy, Clone)]
pub enum State {
    TopLevel,
    InText,
}
