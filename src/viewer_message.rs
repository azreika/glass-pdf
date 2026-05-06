
#[derive(Clone, Debug)]
pub enum Message {
    DrawText { x_pos: i32, y_pos: i32, str: String, size: f32 },
    DrawBlock(Vec<Message>),
    Noop,
}

#[derive(Copy, Clone)]
pub enum State {
    TopLevel,
    InText,
}
