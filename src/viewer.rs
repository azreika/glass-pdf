use std::num::NonZeroU32;
use std::sync::Arc;

use softbuffer::{Context, Surface};
use tiny_skia::{Color, FillRule, Mask, Path, Pixmap};
use winit::application::ApplicationHandler;
use winit::event::{MouseScrollDelta, WindowEvent};
use winit::window::Window;

use crate::content::streamer::{ClippingRule, ContentStreamer, PathPiece};
use crate::content::tokenizer::Token;
use crate::fonts::{Font, FontLib};
use crate::pdf::ast::{ColourSpace, ColourSpaceLib};
use crate::viewer_message::{GlyphInfo, Message, PathInfo};

#[derive(Clone, Debug)]
pub struct PageCtx {
    pub height: f64,
    pub width: f64,
    pub font_lib: FontLib,
    pub cs_lib: ColourSpaceLib,
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
use winit::event_loop::EventLoop;
use winit::window::WindowId;

pub fn view_contents(page_ctx: &PageCtx, tokens: &Vec<Token>) {
    let event_loop = EventLoop::new().unwrap();
    event_loop.run_app(&mut App::new(page_ctx, tokens)).unwrap();
}

struct App {
    window: Option<Arc<Window>>,
    surface: Option<Surface<Arc<Window>, Arc<Window>>>,
    ctx: PageCtx,
    shapes: Vec<PathInfo>,
    glyphs: Vec<GlyphInfo>,
    width: u32,
    height: u32,

    zoom_scale: f64,
    cached_scale_factor: f32,
    rasterized_glyphs: Vec<RasterGlyphPix>,

    base_pixmap: Option<tiny_skia::Pixmap>,
    out_pixmap: Option<tiny_skia::Pixmap>,
}

impl App {
    fn new(ctx: &PageCtx, toks: &Vec<Token>) -> Self {
        let messages = ContentStreamer::process_stream(ctx, toks);

        let mut shapes = vec![];
        let mut glyphs = vec![];

        for msg in messages.into_iter() {
            match msg {
                Message::DrawGlyph(info) => glyphs.push(info),
                Message::DrawPath(info) => shapes.push(info),
                Message::DrawBlock(_) => panic!("unexpected draw block in messages"),
                Message::Noop => panic!("unexpected noop in messages"),
            }
        }

        return App {
            window: None,
            surface: None,
            ctx: ctx.clone(),
            shapes,
            glyphs,
            width: ctx.width as u32,
            height: ctx.height as u32,
            zoom_scale: 1.0,

            out_pixmap: None,
            base_pixmap: None,
            cached_scale_factor: 0.0,
            rasterized_glyphs: vec![],
        }
    }

    fn draw_to_pixmap(&mut self) {
        let output = self.out_pixmap.as_mut().unwrap();
        let base = self.base_pixmap.as_ref().unwrap();

        output.fill(tiny_skia::Color::from_rgba(0.8, 0.8, 0.8, 1.0).unwrap());
        output.draw_pixmap(
            0, 0,
            base.as_ref(),
            &tiny_skia::PixmapPaint::default(),
            tiny_skia::Transform::from_scale(self.zoom_scale as f32, self.zoom_scale as f32),
            None,
        );
    }

    fn to_skia_path(&self, path: &Vec<PathPiece>) -> Option<Path> {
        let mut pb = tiny_skia::PathBuilder::new();
        for piece in path {
            match piece {
                PathPiece::MoveTo { x, y } => pb.move_to(*x as f32, *y as f32),
                PathPiece::LineTo { x, y } => pb.line_to(*x as f32, *y as f32),
                PathPiece::Close => pb.close(),
                PathPiece::Rect { x, y, w, h } => {
                    pb.push_rect(
                        tiny_skia::Rect::from_xywh(*x as f32, *y as f32, *w as f32, *h as f32).unwrap()
                    );
                }
            }
        }
        return pb.finish();
    }

    fn phys_w(&self) -> u32 {
        let sf = self.cached_scale_factor as f64;
        return (self.ctx.width * sf) as u32;
    }

    fn phys_h(&self) -> u32 {
        let sf = self.cached_scale_factor as f64;
        return (self.ctx.height * sf) as u32;
    }

    fn mk_mask(&self, clips: &Vec<(ClippingRule,Vec<PathPiece>)>) -> Mask {
        let sf = self.cached_scale_factor as f64;
        let mut base_map = Pixmap::new(self.phys_w(), self.phys_h()).unwrap();
        base_map.fill(Color::WHITE);

        let mut mask = Mask::new(self.phys_w(), self.phys_h()).unwrap();
        let transform = tiny_skia::Transform::from_scale(sf as f32, sf as f32);

        for (rule, clip) in clips {
            let skia_rule = match rule {
                ClippingRule::EvenOdd => FillRule::EvenOdd,
                ClippingRule::NonWinding => FillRule::Winding,
            };

            let mask_path = self.to_skia_path(&clip).unwrap();
            mask.clear();
            mask.fill_path(&mask_path, skia_rule, true, transform);
            base_map.apply_mask(&mask);
        }

        return Mask::from_pixmap(base_map.as_ref(), tiny_skia::MaskType::Alpha);
    }

    fn build_base_pixmap(&mut self) -> Pixmap {
        let sf = self.cached_scale_factor as f64;

        let phys_w = self.phys_w();
        let phys_h = self.phys_h();
        let mut pixmap = tiny_skia::Pixmap::new(phys_w, phys_h).unwrap();

        pixmap.fill(tiny_skia::Color::from_rgba(0.8, 0.8, 0.8, 1.0).unwrap());
        let transform = tiny_skia::Transform::from_scale(sf as f32, sf as f32);

        for info in &self.shapes {
            let path = self.to_skia_path(&info.path).unwrap();
            let col = to_skia_colour(&info.colour);
            let mut paint = tiny_skia::Paint::default();
            paint.set_color(col);
            let rule = match info.rule {
                ClippingRule::NonWinding => tiny_skia::FillRule::Winding,
                ClippingRule::EvenOdd => tiny_skia::FillRule::EvenOdd,
            };
            let mask = self.mk_mask(&info.clips);
            pixmap.fill_path(&path, &paint, rule, transform, Some(&mask));
        }

        for glyph in &self.rasterized_glyphs {
            let mut glyph_pixmap = tiny_skia::Pixmap::new(glyph.w as u32, glyph.h as u32).unwrap();
            for (dst, src) in glyph_pixmap
                .pixels_mut()
                .iter_mut()
                .zip(glyph.rgba.chunks_exact(4))
            {
                let a = src[3] as f32 / 255.0;
                // premultiply once, here, when writing into the Pixmap
                *dst = tiny_skia::PremultipliedColorU8::from_rgba(
                    (src[0] as f32 * a) as u8,
                    (src[1] as f32 * a) as u8,
                    (src[2] as f32 * a) as u8,
                    src[3],
                )
                .unwrap();
            }

            pixmap.draw_pixmap(
                glyph.x as i32,
                glyph.y as i32,
                glyph_pixmap.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                tiny_skia::Transform::identity(),
                None,
            );
        }

        return pixmap;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(Window::default_attributes().with_title("PDF"))
                .unwrap(),
        );
        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();
        let size = window.inner_size();
        self.width = size.width;
        self.height = size.height;

        let scale_factor = window.scale_factor() as f32;
        let rasterized_glyphs =
                rasterize_glyph_pixels(&self.glyphs, scale_factor as f64, &self.ctx);

        self.cached_scale_factor = scale_factor;

        self.window = Some(window);
        self.surface = Some(surface);

        self.rasterized_glyphs = rasterized_glyphs;
        self.base_pixmap = Some(self.build_base_pixmap());
        self.out_pixmap = Some(tiny_skia::Pixmap::new(self.width, self.height).unwrap());
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: WindowId,
        event: winit::event::WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                assert_eq!(self.window.as_ref().unwrap().scale_factor(), self.cached_scale_factor as f64);
                self.draw_to_pixmap();
                let pixmap = self.out_pixmap.as_ref().unwrap();
                let surface = self.surface.as_mut().unwrap();

                surface
                    .resize(
                        NonZeroU32::new(self.width).unwrap(),
                        NonZeroU32::new(self.height).unwrap(),
                    )
                    .unwrap();
                let mut buf = surface.buffer_mut().unwrap();
                for (pixel, src) in buf.iter_mut().zip(pixmap.data().chunks_exact(4)) {
                    *pixel = ((src[0] as u32) << 16) | ((src[1] as u32) << 8) | src[2] as u32;
                }
                buf.present().unwrap();
            }

            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    self.width = size.width;
                    self.height = size.height;
                    self.out_pixmap = Some(tiny_skia::Pixmap::new(self.width, self.height).unwrap());
                    self.window.as_ref().unwrap().request_redraw();
                }
            },

            WindowEvent::MouseWheel { device_id: _, delta, phase: _ } => {
                let y = match delta {
                    MouseScrollDelta::LineDelta(y, .. ) => y as f64,
                    MouseScrollDelta::PixelDelta(y, ..) => y.y,
                };
                if y != 0.0 {
                    self.zoom_scale *= 1.0 + y * 0.02;
                    self.zoom_scale = self.zoom_scale.clamp(0.1, 10.0);
                    self.window.as_ref().unwrap().request_redraw();
                }
            },
            _ => {}
        }
    }
}

fn to_skia_colour(colour: &Option<Vec<f64>>) -> tiny_skia::Color {
    match colour {
        None => tiny_skia::Color::BLACK,
        Some(vv) if vv.len() == 3 => {
            tiny_skia::Color::from_rgba(vv[0] as f32, vv[1] as f32, vv[2] as f32, 1.0).unwrap()
        }
        Some(vv) if vv.len() == 1 => {
            let g = vv[0] as f32;
            tiny_skia::Color::from_rgba(g, g, g, 1.0).unwrap()
        }
        _ => tiny_skia::Color::BLACK,
    }
}

fn colourize_bitmap(bitmap: &Vec<u8>, colour: &Option<Vec<f64>>) -> Vec<u8> {
    match colour {
        Some(vv) => {
            if vv.len() == 3 {
                // RGB
                let rgb = vv.iter().map(|a| (a * 255.0) as u8).collect::<Vec<u8>>();
                let rgba: Vec<u8> = bitmap
                    .iter()
                    .flat_map(|&a| {
                        let mut vv = rgb.clone();
                        vv.push(a);
                        return vv;
                    })
                    .collect();
                return rgba;
            } else if vv.len() == 1 {
                let g = (vv[0] * 255.0) as u8;
                // Grayscale
                return bitmap
                    .iter()
                    .flat_map(|&a| {
                        return [g, g, g, a];
                    })
                    .collect();
            } else {
                // CMYK?
                panic!("unexpected length of colour: {}", bitmap.len());
            }
        }
        None => {
            return [0, 0, 0].to_vec();
        }
    };
}

struct RasterGlyphPix {
    rgba: Vec<u8>,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

fn rasterize_glyph_pixels(
    glyphs: &Vec<GlyphInfo>,
    scale_factor: f64,
    ctx: &PageCtx,
) -> Vec<RasterGlyphPix> {
    let mut vv = vec![];
    for info in glyphs.iter() {
        let font = ctx.font_lib.get_font(&info.font_id);
        let glyph_id = font.ttf.lookup_glyph_index(info.byte as char);
        if glyph_id == 0 {
            println!("Glyph not handled: {}", info.byte as char);
            continue;
        }

        let (metrics, bitmap) = font
            .ttf
            .rasterize_indexed(glyph_id, (info.size * scale_factor) as f32);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }

        let rgba = colourize_bitmap(&bitmap, &info.colour);

        let gap = (info.width - metrics.width as f64 / scale_factor) / 2.0;
        let x = info.x + gap;

        let mut y = ctx.height - info.y;
        y -= (metrics.height as i32 + metrics.ymin) as f64 / scale_factor;

        vv.push(RasterGlyphPix {
            rgba,
            x: x * scale_factor,
            y: y * scale_factor,
            w: metrics.width as f64,
            h: metrics.height as f64,
        });
    }
    return vv;
}
