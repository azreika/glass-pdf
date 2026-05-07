use iced::{Color, Element, Task};
use iced;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Length, Point, Renderer, Theme};

use crate::content_tokenizer::ContentToken;
use crate::content_streamer::ContentStreamer;
use crate::fonts::FontLib;
use crate::viewer_message::{Message,GlyphInfo};

pub fn view_contents(font_lib: &FontLib, tokens: &Vec<ContentToken>) {
    let flib = font_lib.clone();
    let toks = tokens.clone();

    let mut app = iced::application(
        move || {
            let stream = ContentStreamer::stream_content(flib.clone(), toks.clone());
            let task = Task::stream(stream);
            (Viewer { glyphs: vec![] }, task)
        },
        Viewer::update,
        Viewer::view
    );

    for (_, font) in font_lib.id_to_font.iter() {
        app = app.font(font.font_bytes.clone());
    }

    app.run().unwrap();
}

struct Viewer {
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
            Message::Noop => panic!("Noops should have been filtered out"),
        }
    }

    fn view(&self) -> Element<'_, Message> {
        return Canvas::new(Page {
            padding_x: 40.0,
            padding_y: 20.0,
            glyphs: self.glyphs.clone(),
        })
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }
}

#[derive(Clone, Debug)]
struct TextInfo {
    x: i32,
    txt: String,
    size: f32,
}

struct Page {
    padding_x: f32,
    padding_y: f32,
    glyphs: Vec<GlyphInfo>
}

impl <Message> canvas::Program<Message> for Page {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: iced::Rectangle,
        _cursor: iced::mouse::Cursor,
    ) -> Vec<Geometry> {

        // TODO: these shouldnt be constants
        let page_width = 612.0;
        let page_height = 792.0;
        let scale_factor = 2.0;

        let mut geom: Vec<Geometry> = vec![];

        // outer rectangle
        let mut f1 = Frame::new(renderer, bounds.size());
        let outer_rect = canvas::Path::rectangle(Point { x: 0.0, y: 0.0 }, bounds.size());
        f1.fill(&outer_rect, Color::from_rgb(0.8, 0.8, 0.8));
        geom.push(f1.into_geometry());

        // inner rectangle
        let mut f2 = Frame::new(renderer, bounds.size());
        let inner_size = iced::Size {
            width: page_width,
            height: page_height,
        };

        let inner_rect = canvas::Path::rectangle(Point { x: self.padding_x, y: self.padding_y}, inner_size);
        f2.fill(&inner_rect, Color::from_rgb(1.0, 1.0, 1.0));
        geom.push(f2.into_geometry());

        for info in self.glyphs.iter() {
            let mut frame = Frame::new(renderer, bounds.size());

            let cc = info.byte;

            let glyph_id = info.font.ttf.lookup_glyph_index(cc as char);
            if glyph_id == 0 {
                // glyph not found, skip or use replacement
                println!("HUH MISSING!?!? {}", cc);
            }

            let (metrics, bitmap) = info.font.ttf.rasterize_indexed(glyph_id, info.size*scale_factor);
            if metrics.width == 0 || metrics.height == 0 {
                continue;
            }

            let rgba: Vec<u8> = bitmap.iter().flat_map(|&a| [0,0,0,a]).collect();
            let handle = iced::widget::image::Handle::from_rgba(
                metrics.width as u32,
                metrics.height as u32,
                rgba,
            );

            let mut y_pos = page_height as f32;
            y_pos -= info.y as f32 + self.padding_y;
            y_pos -= (metrics.height as f32 + metrics.ymin as f32)/scale_factor;

            let x_pos = self.padding_x + info.x as f32;

            frame.draw_image(iced::Rectangle {
                x: x_pos,
                y: y_pos,
                width: (metrics.width as f32)/scale_factor,
                height: (metrics.height as f32)/scale_factor,
            }, &handle);

            geom.push(frame.into_geometry());
        }

        return geom;
    }
}
