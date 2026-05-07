use iced::widget::Action;
use iced::{Color, Element, Task};
use iced;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Length, Point, Renderer, Theme};

use crate::content_tokenizer::ContentToken;
use crate::content_streamer::ContentStreamer;
use crate::fonts::FontLib;
use crate::viewer_message::{Message,GlyphInfo};

#[derive(Clone,Debug)]
pub struct PageCtx {
    pub height: f64,
    pub width: f64,
    pub font_lib: FontLib,
    pub scale_factor: f64,
}

pub fn view_contents(page_ctx: &PageCtx, tokens: &Vec<ContentToken>) {
    let ctx = page_ctx.clone();
    let toks = tokens.clone();
    let flib = ctx.font_lib.clone();
    let fflib = flib.clone();

    let mut app = iced::application(
        move || {
            let stream = ContentStreamer::stream_content(flib.clone(), toks.clone());
            let task = Task::stream(stream);
            let scale_task = iced::window::oldest()
                .then(|id| iced::window::scale_factor(id.unwrap()))
                .map(Message::SetScaleFactor);
            (Viewer { ctx: ctx.clone(), glyphs: vec![] }, Task::batch([task, scale_task]))
        },
        Viewer::update,
        Viewer::view
    );

    for (_, font) in fflib.id_to_font.iter() {
        app = app.font(font.font_bytes.clone());
    }

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
                self.ctx.scale_factor = x as f64;
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

struct PageState {
    scale: f64,
}

impl Default for PageState {
    fn default() -> Self {
        return PageState {
            scale: 1.0,
        }
    }
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
        let scale_factor = self.ctx.scale_factor;

        let mut geom: Vec<Geometry> = vec![];

        // outer rectangle
        let mut f1 = Frame::new(renderer, bounds.size());
        let outer_rect = canvas::Path::rectangle(Point { x: 0.0, y: 0.0 }, bounds.size());
        f1.fill(&outer_rect, Color::from_rgb(0.8, 0.8, 0.8));
        geom.push(f1.into_geometry());

        // inner rectangle
        let mut f2 = Frame::new(renderer, bounds.size());
        let inner_size = iced::Size {
            width: (page_width * state.scale) as f32,
            height: (page_height * state.scale) as f32,
        };

        let inner_rect = canvas::Path::rectangle(Point { x: (self.padding_x * state.scale) as f32, y: (self.padding_y * state.scale) as f32}, inner_size);
        f2.fill(&inner_rect, Color::from_rgb(1.0, 1.0, 1.0));
        geom.push(f2.into_geometry());

        for info in self.glyphs.iter() {
            let mut frame = Frame::new(renderer, bounds.size());
            let cc = info.byte;
            let font = self.ctx.font_lib.get_font(&info.font_id);
            let glyph_id = font.ttf.lookup_glyph_index(cc as char);
            assert_ne!(glyph_id, 0);

            let (metrics, bitmap) = font.ttf.rasterize_indexed(glyph_id, (info.size*scale_factor) as f32);
            if metrics.width == 0 || metrics.height == 0 {
                continue;
            }
            // assert!(info.width/2.0 >= metrics.width as f64);
            let gap = (info.width - metrics.width as f64 / scale_factor) / 2.0;

            let rgba: Vec<u8> = bitmap.iter().flat_map(|&a| [0,0,0,a]).collect();
            let handle = iced::widget::image::Handle::from_rgba(
                metrics.width as u32,
                metrics.height as u32,
                rgba,
            );

            let mut y_pos = page_height;
            y_pos -= info.y + self.padding_y;
            y_pos -= (metrics.height as i32 + metrics.ymin) as f64/scale_factor;

            let x_pos = self.padding_x + info.x + gap;

            let screen_x = x_pos * state.scale;
            let screen_y = y_pos * state.scale;

            frame.draw_image(iced::Rectangle {
                x: screen_x as f32,
                y: screen_y as f32,
                width: ((metrics.width as f64)/scale_factor * state.scale) as f32,
                height: ((metrics.height as f64)/scale_factor * state.scale) as f32,
            }, &handle);

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
        match event {
            iced::Event::Mouse(iced::mouse::Event::WheelScrolled { delta }) => {
                match delta {
                    iced::mouse::ScrollDelta::Lines { y, .. }
                    | iced::mouse::ScrollDelta::Pixels { y, .. } => {
                        if *y == 0.0 {
                            return None;
                        }
                        state.scale *= 1.0 + *y as f64 * 0.02;
                        state.scale = state.scale.clamp(0.1, 10.0);
                        return Some(Action::request_redraw());
                    },
                }
            }
            _ => {}
        }
        return None;
    }
}
