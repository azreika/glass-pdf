use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use tiny_skia::{FillRule, Mask, Pixmap, PixmapPaint, PremultipliedColorU8, Stroke, Transform};
use tiny_skia::{Color as SkiaColor, Rect as SkiaRect, Path as SkiaPath};
use winit::application::ApplicationHandler;
use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::window::Window;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowId;

use crate::content::graphics::{ClippingRule, PathOp};
use crate::content::streamer::ContentStreamer;
use crate::content::tokenizer::Token;
use crate::fonts::{Font, FontLib};
use crate::pdf::ast::{ColourSpace, ColourSpaceLib, XObjectLib};
use crate::view_info::ViewInfo;
use crate::viewer_message::{Color, GlyphInfo, Message, PaintType, PathInfo, XObjectInfo};


#[derive(Clone, Debug)]
pub struct PageCtx {
    pub height: f64,
    pub width: f64,
    pub font_lib: FontLib,
    pub cs_lib: ColourSpaceLib,
    pub xobj_lib: XObjectLib,
}

impl PageCtx {
    #[allow(unused)]
    pub fn add_colourspace(&mut self, id: String, cs: ColourSpace) {
        self.cs_lib.id_to_cs.insert(id, cs);
    }

    #[allow(unused)]
    pub fn add_font(&mut self, font: Font) {
        self.font_lib.id_to_font.insert(font.id.to_string(), font);
    }
}

pub fn view_contents(page_ctx: &PageCtx, tokens: &Vec<Token>) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::new(page_ctx, tokens)).unwrap();
}

struct WindowState {
    window: Arc<Window>,
    surface: Surface<Arc<Window>, Arc<Window>>,
}

struct ObjectInfo {
    shapes: Vec<PathInfo>,
    glyphs: Vec<GlyphInfo>,
    xobjs: Vec<XObjectInfo>,
}

impl WindowState {
    fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("PDF"))
                .unwrap(),
        );
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();
        return WindowState { window, surface };
    }

    fn size(&self) -> (u32, u32) {
        let sz = self.window.inner_size();
        return (sz.width, sz.height);
    }

    fn request_redraw(&self) {
        return self.window.request_redraw();
    }

    fn scale_factor(&self) -> f64 {
        return self.window.scale_factor();
    }
}

struct App {
    ctx: PageCtx,
    view: ViewInfo,
    objects: ObjectInfo,

    window: Option<WindowState>,
    base_pixmap: Option<Pixmap>,
}

fn pixmap_color(src: &[u8]) -> PremultipliedColorU8 {
    let alpha = if src.len() == 4 {
        src[3]
    } else if src.len() == 1 {
        let g = (src[0] as f32) as u8;
        return PremultipliedColorU8::from_rgba(g,g,g,255).unwrap();
    } else {
        assert_eq!(src.len(), 3);
        255u8
    };

    let a = alpha as f32 / 255.0;

    return PremultipliedColorU8::from_rgba(
        (src[0] as f32 * a) as u8,
        (src[1] as f32 * a) as u8,
        (src[2] as f32 * a) as u8,
        alpha,
    ).unwrap();
}

fn draw_pixels(pixmap: &mut Pixmap, view_box: ViewBox, data: &Vec<u8>, chunksize: usize, t: Transform) {
    let mut inner_pixmap = Pixmap::new(view_box.w, view_box.h).unwrap();
    for (dst, src) in inner_pixmap.pixels_mut().iter_mut().zip(data.chunks_exact(chunksize)) {
        *dst = pixmap_color(&src);
    }
     pixmap.draw_pixmap(
        view_box.x as i32, view_box.y as i32,
        inner_pixmap.as_ref(),
        &PixmapPaint::default(),
        t,
        None,
    );
}

fn process_messages(messages: Vec<Message>) -> ObjectInfo {
    let mut shapes = vec![];
    let mut glyphs = vec![];
    let mut xobjs = vec![];

    for msg in messages.into_iter() {
        match msg {
            Message::DrawGlyph(info) => glyphs.push(info),
            Message::DrawPath(info) => shapes.push(info),
            Message::StrokePath(info) => shapes.push(info),
            Message::DrawXObject(info) => xobjs.push(info),
            Message::DrawBlock(_) => panic!("unexpected draw block in messages"),
            Message::Noop => panic!("unexpected noop in messages"),
        }
    }

    return ObjectInfo { shapes, glyphs, xobjs };
}

impl App {
    fn new(ctx: &PageCtx, toks: &Vec<Token>) -> Self {
        let messages = ContentStreamer::process_stream(ctx, toks);
        let objects = process_messages(messages);

        return App {
            window: None,
            ctx: ctx.clone(),
            objects,
            view: ViewInfo::new(),
            base_pixmap: None,
        }
    }

    fn window(&self) -> &WindowState {
        return self.window.as_ref().unwrap();
    }

    fn draw_to_pixmap(&mut self) -> Pixmap {
        let sz = self.window().size();
        let mut output = Pixmap::new(sz.0, sz.1).unwrap();
        let base = self.base_pixmap.as_ref().unwrap();
        output.fill(SkiaColor::from_rgba(0.8, 0.8, 0.8, 1.0).unwrap());
        output.draw_pixmap(
            0, 0,
            base.as_ref(),
            &PixmapPaint::default(),
            Transform::from(&self.view),
            None,
        );
        return output;
    }

    fn to_skia_path(&self, path: &Vec<PathOp>) -> Option<SkiaPath> {
        let mut pb = tiny_skia::PathBuilder::new();
        for piece in path.iter() {
            match *piece {
                PathOp::MoveTo { x, y } => pb.move_to(x as f32, y as f32),
                PathOp::LineTo { x, y } => pb.line_to(x as f32, y as f32),
                PathOp::Close => pb.close(),
                PathOp::Rect { x, y, w, h } => {
                    pb.push_rect(
                        SkiaRect::from_xywh(x as f32, y as f32, w as f32, h as f32).unwrap()
                    );
                }
            }
        }
        return pb.finish();
    }

    fn handle_close(&self, event_loop: &winit::event_loop::ActiveEventLoop) {
        event_loop.exit();
    }

    fn handle_resize(&mut self, size: PhysicalSize<u32>) {
        if size.width > 0 && size.height > 0 {
            let window_state = self.window.as_mut().unwrap();
            window_state.surface.resize(
                NonZeroU32::new(size.width).unwrap(),
                NonZeroU32::new(size.height).unwrap(),
            ).unwrap();
            self.redraw();
        }
    }

    fn handle_cursor(&mut self, position: PhysicalPosition<f64>) {
        self.view.move_cursor(position.x as f32, position.y as f32);
        if self.view.is_panning {
            self.redraw();
        }
    }

    fn handle_sf_change(&mut self, _: f64) {
        self.revalidate_base();
    }

    fn revalidate_base(&mut self) {
        self.base_pixmap = Some(self.mk_base_pixmap());
    }

    fn handle_click(&mut self, state: ElementState, button: MouseButton) {
        match button {
            MouseButton::Left => {
                self.view.toggle_panning(state == winit::event::ElementState::Pressed);
            },
            _ => {},
        }
    }

    fn handle_scroll(&mut self, delta: MouseScrollDelta) {
        let y = match delta {
            MouseScrollDelta::LineDelta(y, .. ) => y,
            MouseScrollDelta::PixelDelta(y, ..) => y.y as f32,
        };
        if y == 0.0 { return; }
        self.view.zoom_in(y);
        self.redraw();
    }

    fn handle_redraw(&mut self) {
        let pixmap = self.draw_to_pixmap();
        let window_state = self.window.as_mut().unwrap();
        let mut buf = window_state.surface.buffer_mut().unwrap();
        for (pixel, src) in buf.iter_mut().zip(pixmap.data().chunks_exact(4)) {
            *pixel = ((src[0] as u32) << 16) | ((src[1] as u32) << 8) | src[2] as u32;
        }
        buf.present().unwrap();
    }

    fn phys_w(&self) -> u32 {
        let sf = self.scale_factor();
        return (self.ctx.width * sf) as u32;
    }

    fn phys_h(&self) -> u32 {
        let sf = self.scale_factor();
        return (self.ctx.height * sf) as u32;
    }

    fn scale_factor(&self) -> f64 {
        return self.window().scale_factor();
    }

    fn mk_mask(&self, clips: &Vec<(ClippingRule,Vec<PathOp>)>) -> Mask {
        let mut base_map = Pixmap::new(self.phys_w(), self.phys_h()).unwrap();
        base_map.fill(SkiaColor::WHITE);

        let mut mask = Mask::new(self.phys_w(), self.phys_h()).unwrap();

        for (rule, clip) in clips {
            let skia_rule = FillRule::from(rule);
            let mask_path = self.to_skia_path(&clip).unwrap();
            mask.clear();
            mask.fill_path(&mask_path, skia_rule, true, self.sf_transform());
            base_map.apply_mask(&mask);
        }

        return Mask::from_pixmap(base_map.as_ref(), tiny_skia::MaskType::Alpha);
    }

    fn fill_path(&self, pixmap: &mut Pixmap, path: &Vec<PathOp>, colour: SkiaColor, rule: ClippingRule, mask: Option<&Mask>) {
        let path = self.to_skia_path(path).unwrap();

        let mut paint = tiny_skia::Paint::default();
        paint.set_color(colour);

        let transform = self.sf_transform();
        pixmap.fill_path(&path, &paint, FillRule::from(&rule), transform, mask);
    }

    fn sf_transform(&self) -> Transform {
        let sf = self.scale_factor();
        return Transform::from_scale(sf as f32, sf as f32);
    }

    fn draw_shapes(&self, pixmap: &mut Pixmap) {
        for info in &self.objects.shapes {
            let mask = self.mk_mask(&info.clips);
            let col = SkiaColor::from(&info.colour);
            match info.paint_type {
                PaintType::Fill => {
                    self.fill_path(pixmap, &info.path, col, info.rule, Some(&mask));
                },
                PaintType::Stroke => {
                    let mut paint = tiny_skia::Paint::default();
                    paint.set_color(col);
                    let path = self.to_skia_path(&info.path).unwrap();
                    let transform = self.sf_transform();

                    let stroke = Stroke {
                        ..Stroke::default()
                    };
                    pixmap.stroke_path(&path, &paint, &stroke, transform, Some(&mask));
                },
            }
        }
    }

    fn init_pixmap(&self) -> Pixmap {
        let phys_w = self.phys_w();
        let phys_h = self.phys_h();
        let mut pixmap = Pixmap::new(phys_w, phys_h).unwrap();

        let bg_color = SkiaColor::from_rgba(0.8, 0.8, 0.8, 1.0).unwrap();
        pixmap.fill(bg_color);

        return pixmap;
    }

    fn draw_xobjs(&self, pixmap: &mut Pixmap) {
        let sf = self.scale_factor();
        for info in &self.objects.xobjs {
            let x = info.x * sf;

            let y = (self.ctx.height - info.y - info.y_scale as f64) * sf;

            let x_scale = (info.x_scale as f64 * sf) as f32 / info.w as f32;
            let y_scale = (info.y_scale as f64 * sf) as f32 / info.h as f32;

            let transform = Transform::from_row(
                x_scale,
                0.0, 0.0,
                y_scale,
                x as f32,
                y as f32);

            let smask = &info.smask.clone().unwrap();
            let view_box = ViewBox::new(0.0, 0.0, info.w, info.h);

            let mut xobj_pixmap = Pixmap::new(self.phys_w(), self.phys_h()).unwrap();
            let mut mask_pixmap = Pixmap::new(self.phys_w(), self.phys_h()).unwrap();
            draw_pixels(&mut mask_pixmap, view_box, &smask.bytes, 1, transform);

            let mask = Mask::from_pixmap(mask_pixmap.as_ref(), tiny_skia::MaskType::Luminance);
            draw_pixels(
                &mut xobj_pixmap, view_box,
                &info.bytes, 3,
                transform,
            );

            xobj_pixmap.apply_mask(&mask);
            pixmap.draw_pixmap(
                0, 0,
                xobj_pixmap.as_ref(),
                &PixmapPaint::default(),
                Transform::identity(),
                None,
            );
        }
    }

    fn draw_glyphs(&self, pixmap: &mut Pixmap) {
        let rasterized_glyphs = rasterize_glyphs(&self.objects.glyphs, self.scale_factor(), &self.ctx);
        for glyph in rasterized_glyphs {
            let view_box = ViewBox::new(
                glyph.x, glyph.y,
                glyph.w as u32, glyph.h as u32,
            );

            draw_pixels(
                pixmap, view_box,
                &glyph.rgba, 4,
                Transform::identity(),
            );
        }
    }

    fn mk_base_pixmap(&self) -> Pixmap {
        let mut pixmap = self.init_pixmap();
        self.draw_shapes(&mut pixmap);
        self.draw_glyphs(&mut pixmap);
        self.draw_xobjs(&mut pixmap);
        return pixmap;
    }

    fn redraw(&self) {
        self.window().request_redraw();
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        self.window = Some(WindowState::new(event_loop));
        self.revalidate_base();
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _: WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => self.handle_redraw(),
            WindowEvent::CloseRequested => self.handle_close(event_loop),
            WindowEvent::Resized(size) => self.handle_resize(size),
            WindowEvent::MouseWheel { delta, .. } => self.handle_scroll(delta),
            WindowEvent::MouseInput { state, button, .. } => self.handle_click(state, button),
            WindowEvent::CursorMoved { position, .. } => self.handle_cursor(position),
            WindowEvent::ScaleFactorChanged { scale_factor: sf, .. } => self.handle_sf_change(sf),
            _ => {}
        }
    }
}

impl From<&Color> for SkiaColor {
    fn from(colour: &Color) -> SkiaColor {
        match *colour {
            Color::Default => SkiaColor::BLACK,
            Color::RGB(r,g,b) => SkiaColor::from_rgba(r, g, b, 1.0).unwrap(),
            Color::RGBA(r,g,b,a) => SkiaColor::from_rgba(r, g, b, a).unwrap(),
            Color::Gray(g) => SkiaColor::from_rgba(g, g, g, 1.0).unwrap(),
        }
    }
}

fn alpha_bitmap_to_rgba(bitmap: &[u8], rgb8: [u8; 3]) -> Vec<u8> {
    return bitmap.iter().flat_map(|&a| {
        return [rgb8[0], rgb8[1], rgb8[2], a]
    }).collect();
}

impl From<&ClippingRule> for FillRule {
    fn from(rule: &ClippingRule) -> FillRule {
        return match rule {
            ClippingRule::Winding => FillRule::Winding,
            ClippingRule::EvenOdd => FillRule::EvenOdd,
        };
    }
}

struct RasterizedGlyph {
    rgba: Vec<u8>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Copy, Clone, Debug)]
struct ViewBox {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
}

impl ViewBox {
    fn new(x: f64, y: f64, w: u32, h: u32) -> Self {
        return ViewBox {
            x: x as i32,
            y: y as i32,
            w,
            h,
        };
    }
}

fn rasterize_glyphs(
    glyphs: &Vec<GlyphInfo>,
    scale_factor: f64,
    ctx: &PageCtx,
) -> Vec<RasterizedGlyph> {
    let mut vv = vec![];
    for info in glyphs.iter() {
        let font = ctx.font_lib.get_font(&info.font_id);
        let glyph_id = font.ttf.lookup_glyph_index(info.byte as char);
        if glyph_id == 0 {
            println!("Glyph not handled: {}", info.byte as char);
            continue;
        }

        let (metrics, bitmap) = font.ttf
            .rasterize_indexed(glyph_id, (info.size * scale_factor) as f32);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }

        let rgba = alpha_bitmap_to_rgba(&bitmap, info.color_fill.to_rgb8());

        let gap = (info.width - metrics.width as f64 / scale_factor) / 2.0;
        let x = info.x + gap;

        let mut y = ctx.height - info.y;
        y -= (metrics.height as i32 + metrics.ymin) as f64 / scale_factor;

        vv.push(RasterizedGlyph {
            rgba,
            x: x * scale_factor,
            y: y * scale_factor,
            w: metrics.width as f64,
            h: metrics.height as f64,
        });
    }
    return vv;
}
