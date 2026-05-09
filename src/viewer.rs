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
    pub scale_factor: f64,
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
            let gap = (info.width - metrics.width as f64 / scale_factor) / 2.0;

            let rgba = colourize_bitmap(&bitmap, &info.colour);
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
