use crate::fonts::Font;

#[derive(Clone, Debug)]
pub enum Message {
    DrawBlock(Vec<Message>),
    DrawGlyph(GlyphInfo),
    Noop,
}

#[derive(Clone, Debug)]
pub struct GlyphInfo {
    pub x: i32,
    pub y: i32,
    pub byte: u8,
    pub size: f32,
    pub font: Font,
}

#[derive(Copy, Clone)]
pub enum State {
    TopLevel,
    InText,
}

pub struct GraphicsState {
    pub ctm: Vec<f64>,
}

impl GraphicsState {
    fn init_matrix() -> Vec<f64> {
        return vec![
            1.0, 0.0, 0.0,
            0.0, 1.0, 0.0,
            0.0, 0.0, 1.0
        ];
    }

    pub fn new() -> Self {
        return GraphicsState {
            ctm: Self::init_matrix(),
        };
    }
}
