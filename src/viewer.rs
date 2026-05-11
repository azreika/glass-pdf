use iced::widget::Action;
use iced::{Color, Element, Task};
use iced;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Length, Point, Renderer, Theme};

use crate::content::tokenizer::Token;
use crate::content::streamer::{ContentStreamer, stream_content};
use crate::fonts::{Font, FontLib};
use crate::pdf::ast::{ColourSpace, ColourSpaceLib};
use crate::viewer_message::{Message,GlyphInfo};

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
            (Viewer { ctx: ctx.clone(), glyphs: vec![] }, Task::batch([task, scale_task]))
        },
        Viewer::update,
        Viewer::view
    );
    app.run().unwrap();
}

struct Viewer {
    ctx: PageCtx,
    glyphs: Vec<GlyphInfo>,
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
            Message::Noop => panic!("Noops should have been filtered out"),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        return Canvas::new(Page {
            padding_x: 40.0,
            padding_y: 20.0,
            glyphs: self.glyphs.clone(),
            ctx: self.ctx.clone(),
        })
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
}

struct Page {
    padding_x: f64,
    padding_y: f64,
    glyphs: Vec<GlyphInfo>,
    ctx: PageCtx,
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
                panic!();
            }
        },
        None => {
            return [0,0,0].to_vec();
        }
    };
}

impl Page {
    fn gen_viewer_background(&self, renderer: &Renderer, bounds: iced::Rectangle) -> Geometry {
        let mut f1 = Frame::new(renderer, bounds.size());
        let outer_rect = canvas::Path::rectangle(Point { x: 0.0, y: 0.0 }, bounds.size());
        f1.fill(&outer_rect, Color::from_rgb(0.8, 0.8, 0.8));
        return f1.into_geometry();
    }
}

fn rasterize_glyphs(glyphs: &Vec<GlyphInfo>, scale_factor: f64, ctx: &PageCtx) -> Vec<RasterizedGlyph> {
    let mut vv = vec![];
    for info in glyphs.iter() {
        let cc = info.byte;
        let font = ctx.font_lib.get_font(&info.font_id);

        let glyph_id = font.ttf.lookup_glyph_index(cc as char);
        assert_ne!(glyph_id, 0);

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

        let mut y = ctx.height;
        y -= info.y;
        y -= (metrics.height as i32 + metrics.ymin) as f64/scale_factor;

        vv.push(RasterizedGlyph { handle, x, y, w: metrics.width as f64, h: metrics.height as f64 })
    }
    return vv;
}

fn refresh_glyphs(state: &mut PageState, glyphs: &Vec<GlyphInfo>, ctx: &PageCtx) {
    let rglyphs = rasterize_glyphs(glyphs, state.cached_scale_factor, ctx);
    state.cached_glyph_count = glyphs.len();
    state.rasterized = rglyphs;
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
        // TODO: these shouldnt be constants
        let page_width = self.ctx.width;
        let page_height = self.ctx.height;
        let scale_factor = self.ctx.window_scale_factor;

        let padding_x = self.padding_x * state.zoom_scale;
        let padding_y = self.padding_y * state.zoom_scale;

        let mut geom: Vec<Geometry> = vec![];

        // outer rectangle
        geom.push(self.gen_viewer_background(renderer, bounds));

        // inner rectangle
        let mut f2 = Frame::new(renderer, bounds.size());
        let inner_size = iced::Size {
            width: (page_width * state.zoom_scale) as f32,
            height: (page_height * state.zoom_scale) as f32,
        };

        let inner_rect = canvas::Path::rectangle(Point { x: padding_x as f32, y:  padding_y as f32 }, inner_size);
        f2.fill(&inner_rect, Color::from_rgb(1.0, 1.0, 1.0));
        geom.push(f2.into_geometry());

        let rglyphs = &state.rasterized;

        for info in rglyphs.iter() {
            let mut frame = Frame::new(renderer, bounds.size());

            let screen_x = info.x * state.zoom_scale;
            let screen_y = info.y * state.zoom_scale;

            frame.draw_image(iced::Rectangle {
                x: screen_x as f32 + padding_x as f32,
                y: screen_y as f32 + padding_y as f32,
                width: (info.w/scale_factor * state.zoom_scale) as f32,
                height: (info.h/scale_factor * state.zoom_scale) as f32,
            }, &info.handle);

            geom.push(frame.into_geometry());
        }

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
                        if *y == 0.0 {
                            return None;
                        }
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
