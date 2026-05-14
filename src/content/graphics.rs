use crate::{transform::Matrix, viewer_message::{Color, PathInfo}};

#[derive(Clone,Debug)]
pub struct GraphicsState {
    pub ctm: Matrix,

    pub cs_nostroke: Option<String>,
    pub color_fill: Color,

    pub path: Vec<PathOp>,
    pub clips: Vec<(ClippingRule, Vec<PathOp>)>,
}

impl GraphicsState {
    pub fn new() -> Self {
        return GraphicsState {
            ctm: Matrix::new(),
            cs_nostroke: None,
            color_fill: Color::Default,
            path: vec![],
            clips: vec![],
        };
    }

    pub fn move_to(&mut self, x: f64, y: f64) {
        self.path.push(PathOp::MoveTo { x, y });
    }

    pub fn line_to(&mut self, x: f64, y: f64) {
        self.path.push(PathOp::LineTo { x, y });
    }

    pub fn close_path(&mut self) {
        self.path.push(PathOp::Close);
    }

    pub fn set_color_fill(&mut self, color: Color) {
        self.color_fill = color;
    }

    pub fn draw(&mut self, op: PathOp) {
        self.path.push(op);
    }

    pub fn clip_path(&mut self, rule: ClippingRule) {
        self.clips.push((rule, self.path.clone()));
    }

    pub fn clear_path(&mut self) {
        self.path.clear();
    }

    pub fn finish_path(&mut self, rule: ClippingRule) -> PathInfo {
        let info = PathInfo {
            path: self.path.clone(),
            colour: self.color_fill.clone(),
            rule: rule,
            clips: self.clips.clone(),
        };
        self.clear_path();
        return info;
    }
}

#[derive(Clone, Debug)]
pub enum PathOp {
    Rect { x: f64, y: f64, w: f64, h: f64 },
    MoveTo { x: f64, y: f64 },
    LineTo { x: f64, y: f64 },
    Close,
}

#[derive(Clone, Debug, Copy)]
pub enum ClippingRule {
    Winding,
    EvenOdd,
}
