use iced::widget::Action;
use iced::{Color, Element, Task};
use iced;
use iced::widget::canvas::{self, Canvas, Fill, Frame, Geometry, Path};
use iced::{Length, Point, Renderer, Theme};

use crate::content::tokenizer::Token;
use crate::content::streamer::{ClippingRule, ContentStreamer, PathPiece, stream_content};
use crate::fonts::{Font, FontLib};
use crate::pdf::ast::{ColourSpace, ColourSpaceLib};
use crate::viewer_message::{GlyphInfo, Message, PathInfo};

#[derive(Clone,Debug)]
pub struct PageCtx {
    pub height: f64,
    pub width: f64,
    pub font_lib: FontLib,
    pub window_scale_factor: f64,
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

pub fn view_contents(page_ctx: &PageCtx, tokens: &Vec<Token>) {
    let ctx = page_ctx.clone();
    let toks = tokens.clone();

    let app = iced::application(
        move || {
            let streamer = ContentStreamer::new(ctx.clone(), toks.clone());
            let stream = stream_content(streamer);
            let task = Task::stream(stream);
            let scale_task = iced::window::oldest()
                .then(|id| iced::window::scale_factor(id.unwrap()))
                .map(Message::SetScaleFactor);
            (Viewer {
                    ctx: ctx.clone(),
                    glyphs: vec![],
                    shapes: vec![],
                    clips: vec![],
                }, Task::batch([task, scale_task]))
        },
        Viewer::update,
        Viewer::view
    );
    app.run().unwrap();
}

struct Viewer {
    ctx: PageCtx,
    glyphs: Vec<GlyphInfo>,
    shapes: Vec<PathInfo>,
    clips: Vec<PathInfo>,
}

impl Viewer {
    fn update(&mut self, message: Message) {
        match message {
            Message::DrawBlock(messages) =>  {
                for message in messages.iter() {
                    self.update(message.clone());
                }
            },
            Message::DrawGlyph(info) => {
                self.glyphs.push(info);
            },
            Message::SetScaleFactor(x) => {
                self.ctx.window_scale_factor = x as f64;
            },
            Message::DrawPath(info) => {
                self.shapes.push(info);
            },
            Message::Noop => panic!("Noops should have been filtered out"),
            Message::Clip(info) => {
                self.clips.push(info);
            },
        }
    }

    fn view(&self) -> Element<'_, Message> {
        return Canvas::new(Page {
            padding_x: 0.0,
            padding_y: 0.0,
            glyphs: self.glyphs.clone(),
            shapes: self.shapes.clone(),
            ctx: self.ctx.clone(),
            clips: self.clips.clone(),
        })
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
}

struct Page {
    ctx: PageCtx,
    padding_x: f64,
    padding_y: f64,
    glyphs: Vec<GlyphInfo>,
    shapes: Vec<PathInfo>,
    clips: Vec<PathInfo>,
}

struct RasterizedGlyph {
    handle: iced::widget::image::Handle,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

struct PageState {
    zoom_scale: f64,

    cached_scale_factor: f64,
    cached_glyph_count: usize,
    rasterized: Vec<RasterizedGlyph>,
}

impl Default for PageState {
    fn default() -> Self {
        return PageState {
            zoom_scale: 1.0,
            cached_scale_factor: 0.0,
            cached_glyph_count: 0,
            rasterized: vec![],
        }
    }
}

impl PageState {
    fn scale(&self, x: f64) -> f32{
        let x_f32: f64 = x.into();
        return (x_f32 as f32) * self.zoom_scale as f32;
    }

    fn scaled_pt(&self, x: f64, y: f64) -> Point{
        return Point {
            x: self.scale(x),
            y: self.scale(y),
        };
    }

    fn scaled_size(&self, w: f64, h: f64) -> iced::Size {
        return iced::Size {
            width: self.scale(w),
            height: self.scale(h),
        };
    }
}

fn colourize_bitmap(bitmap: &Vec<u8>, colour: &Option<Vec<f64>>) -> Vec<u8> {
    match colour {
        Some(vv) => {
            if vv.len() == 3 {
                // RGB
                let rgb = vv.iter().map(|a| (a*255.0) as u8).collect::<Vec<u8>>();
                let rgba: Vec<u8> = bitmap.iter().flat_map(|&a| {
                    let mut vv = rgb.clone();
                    vv.push(a);
                    return vv;
                }).collect();
                return rgba;
            } else if vv.len() == 1 {
                let g = (vv[0] * 255.0) as u8;
                // Grayscale
                return bitmap.iter().flat_map(|&a| {
                    return [g, g, g, a];
                }).collect();
            } else {
                // CMYK?
                panic!("unexpected length of colour: {}", bitmap.len());
            }
        },
        None => {
            return [0,0,0].to_vec();
        }
    };
}

impl Page {
    fn mk_viewer_background(&self, renderer: &Renderer, bounds: iced::Rectangle) -> Geometry {
        let mut f1 = Frame::new(renderer, bounds.size());
        let outer_rect = canvas::Path::rectangle(Point { x: 0.0, y: 0.0 }, bounds.size());
        f1.fill(&outer_rect, Color::from_rgb(0.8, 0.8, 0.8));
        return f1.into_geometry();
    }
}

fn rasterize_glyphs(glyphs: &Vec<GlyphInfo>, scale_factor: f64, ctx: &PageCtx) -> Vec<RasterizedGlyph> {
    let mut vv = vec![];
    for info in glyphs.iter() {
        let font = ctx.font_lib.get_font(&info.font_id);
        let glyph_id = font.ttf.lookup_glyph_index(info.byte as char);
        if glyph_id == 0 {
            println!("Glyph not handled: {}", info.byte as char);
            continue;
        }

        let (metrics, bitmap) = font.ttf.rasterize_indexed(glyph_id, (info.size*scale_factor) as f32);
        if metrics.width == 0 || metrics.height == 0 {
            continue;
        }

        let rgba = colourize_bitmap(&bitmap, &info.colour);
        let handle = iced::widget::image::Handle::from_rgba(
            metrics.width as u32,
            metrics.height as u32,
            rgba,
        );

        let gap = (info.width - metrics.width as f64 / scale_factor) / 2.0;
        let x = info.x + gap;

        let mut y = ctx.height - info.y;
        y -= (metrics.height as i32 + metrics.ymin) as f64/scale_factor;

        let w = metrics.width as f64;
        let h = metrics.height as f64;
        vv.push(RasterizedGlyph { handle, x, y, w, h });
    }
    return vv;
}

fn to_iced_path(state: &PageState, info: &PathInfo) -> Path {
    return iced::widget::canvas::Path::new(|builder| {
        for piece in &info.path {
            match *piece {
                PathPiece::MoveTo { x, y } => {
                    builder.move_to(state.scaled_pt(x, y));
                }
                PathPiece::LineTo { x, y } => {
                    builder.line_to(state.scaled_pt(x, y));
                },
                PathPiece::Close => {
                    builder.close();
                },
                PathPiece::Rect {x, y, w, h} => {
                    builder.rectangle(
                        state.scaled_pt(x, y),
                        state.scaled_size(w, h)
                    );
                },
            }
        }
    });
}

fn refresh_glyphs(state: &mut PageState, glyphs: &Vec<GlyphInfo>, ctx: &PageCtx) {
    let rglyphs = rasterize_glyphs(glyphs, state.cached_scale_factor, ctx);
    state.cached_glyph_count = glyphs.len();
    state.rasterized = rglyphs;
}

fn mk_colour(colour: &Option<Vec<f64>>) -> Color {
    if colour.is_none() {
        return Color::BLACK;
    }

    let vv = &colour.as_ref().unwrap();
    if vv.len() == 3 {
        let mut bb = Color::BLACK;
        bb.r = vv[0] as f32;
        bb.g = vv[1] as f32;
        bb.b = vv[2] as f32;
        return bb;
    }

    assert_eq!(vv.len(), 1);
    let g = vv[0] as f32;

    let mut bb = Color::BLACK;
    bb.r = g;
    bb.g = g;
    bb.b = g;
    return bb;
}

impl <Msg> canvas::Program<Msg> for Page {
    type State = PageState;

    fn draw(
        &self,
        state: &PageState,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut geom: Vec<Geometry> = vec![];
        geom.push(self.mk_viewer_background(renderer, bounds));

        let scale_factor = self.ctx.window_scale_factor;
        let mut frame = Frame::new(renderer, bounds.size());
        for info in state.rasterized.iter() {
            frame.draw_image(iced::Rectangle {
                x:      state.scale(info.x + self.padding_x),
                y:      state.scale(info.y + self.padding_y),
                width:  state.scale(info.w/scale_factor),
                height: state.scale(info.h/scale_factor),
            }, &info.handle);
        }

        for info in self.shapes.iter() {
            let iced_path = to_iced_path(state, info);
            let col = mk_colour(&info.colour);
            let mut fill_style = Fill::default();
            fill_style.style = col.into();
            let rule = match info.rule {
                ClippingRule::NonWinding => iced::widget::canvas::fill::Rule::NonZero,
                ClippingRule::EvenOdd => iced::widget::canvas::fill::Rule::EvenOdd,
            };
            frame.fill(&iced_path, Fill {
                style: col.into(),
                rule,
            });
        }

        for info in self.clips.iter() {
            let iced_path = to_iced_path(state, info);
            let col = Color::from_rgba(0.0, 1.0, 0.0, 0.2);
            let mut fill_style = Fill::default();
            fill_style.style = col.into();
            let clip_rule = match info.rule {
                ClippingRule::NonWinding => iced::widget::canvas::fill::Rule::NonZero,
                ClippingRule::EvenOdd => iced::widget::canvas::fill::Rule::EvenOdd,
            };
            frame.with_save(|frame| {
                frame.fill(&iced_path, Fill {
                    style: Color::TRANSPARENT.into(),
                    rule: clip_rule,
                });
            })
            // frame.fill(&iced_path, fill_style);
            // let col = Color::from_rgba(1.0, 0.0, 0.0, 0.2);
            // frame.fill(&iced_path, col);


            // geom.push(frame.into_geometry());
        }
        geom.push(frame.into_geometry());

        return geom;
    }

    fn update(
        &self,
        state: &mut PageState,
        event: &iced::Event,
        _bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Option<iced::widget::Action<Msg>> {
        if self.ctx.window_scale_factor != state.cached_scale_factor {
            state.cached_scale_factor = self.ctx.window_scale_factor;
            refresh_glyphs(state, &self.glyphs, &self.ctx);
        }

        if self.glyphs.len() != state.cached_glyph_count {
            refresh_glyphs(state, &self.glyphs, &self.ctx);
        }

        match event {
            iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. }
                    | iced::mouse::ScrollDelta::Pixels { y, .. } => {
                        if *y == 0.0 { return None; }
                        state.zoom_scale *= 1.0 + *y as f64 * 0.02;
                        state.zoom_scale = state.zoom_scale.clamp(0.1, 10.0);
                        return Some(Action::request_redraw());
                    },
                }
            }
            _ => {}
        }
        return None;
    }
}
