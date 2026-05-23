use crate::{pdf::ast::BlendMode, transform::Matrix, viewer_message::{Color, PaintType, PathInfo}};

#[derive(Clone,Debug)]
pub struct GraphicsState {
    pub ctm: Matrix,

    pub cs_fill: Option<String>,
    pub color_fill: Color,

    pub cs_stroke: Option<String>,
    pub color_stroke: Color,

    pub path: Vec<PathOp>,
    pub clips: Vec<(ClippingRule, Vec<PathOp>)>,

    pub stroke_ca: f32,
    pub fill_ca: f32,
    pub blend_mode: BlendMode,
    pub alpha_source: bool,
}

impl GraphicsState {
    pub fn new() -> Self {
        return GraphicsState {
            ctm: Matrix::new(),

            cs_fill: None,
            color_fill: Color::Default,

            cs_stroke: None,
            color_stroke: Color::Default,

            path: vec![],
            clips: vec![],

            alpha_source: false,

            stroke_ca: 0.0,
            fill_ca: 0.0,
            blend_mode: BlendMode::Normal,
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

    pub fn set_color_stroke(&mut self, color: Color) {
        self.color_stroke = color;
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

    // TODO: better names!!
    pub fn path_stroke_info(&self) -> PathInfo {
        let info = PathInfo {
            path: self.path.clone(),
            colour: self.color_stroke.clone(),
            rule: ClippingRule::Winding, // TODO: not relevant?
            clips: self.clips.clone(),
            paint_type: PaintType::Stroke,
        };
        return info;
    }

    pub fn finish_path_fill(&mut self, rule: ClippingRule) -> PathInfo {
        let info = PathInfo {
            path: self.path.clone(),
            colour: self.color_fill.clone(),
            rule: rule,
            clips: self.clips.clone(),
            paint_type: PaintType::Fill,
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
