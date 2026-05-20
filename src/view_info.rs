use tiny_skia::Transform;

pub struct ViewInfo {
    pan_x: f32,
    pan_y: f32,
    zoom_scale: f32,
    pub is_panning: bool,
    last_cursor: Option<(f32, f32)>,
}

impl From<&ViewInfo> for Transform {
    fn from(view: &ViewInfo) -> Transform {
        return Transform::from_row(
            view.zoom_scale, 0.0,
            0.0, view.zoom_scale,
            view.pan_x, view.pan_y,
        );
    }
}

impl ViewInfo {
    pub fn new() -> Self {
        return ViewInfo {
            pan_x: 0.0,
            pan_y: 0.0,
            zoom_scale: 1.0,
            is_panning: false,
            last_cursor: None,
        };
    }

    pub fn zoom_in(&mut self, y: f32) {
        let old_zoom = self.zoom_scale;
        self.zoom_scale *= 1.0 + y * 0.02;
        self.zoom_scale = self.zoom_scale.clamp(0.1, 10.0);

        let new_zoom = self.zoom_scale;

        if let Some((cx, cy)) = self.last_cursor {
            let ratio = new_zoom / old_zoom;
            self.pan_x = cx - (cx - self.pan_x) * ratio;
            self.pan_y = cy - (cy - self.pan_y) * ratio;
        }
    }

    pub fn toggle_panning(&mut self, v: bool) {
        self.is_panning = v;
        if !v {
            self.last_cursor = None;
        };
    }

    pub fn move_cursor(&mut self, x: f32, y: f32) {
        if self.is_panning {
            if let Some((last_x, last_y)) = self.last_cursor {
                self.pan_x += x - last_x;
                self.pan_y += y - last_y;
            }
        }
        self.last_cursor = Some((x, y));
    }
}
