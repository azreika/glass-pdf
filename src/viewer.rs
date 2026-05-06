use std::collections::HashMap;

use iced::{Color, Element, Task};
use iced;
use iced::widget::canvas::{self, Canvas, Frame, Geometry};
use iced::{Length, Point, Renderer, Theme};

use crate::content_tokenizer::ContentToken;
use crate::content_streamer::ContentStreamer;
use crate::ast::FontLib;
use crate::viewer_message::{Message,GlyphInfo};

pub fn view_contents(font_lib: &FontLib, tokens: &Vec<ContentToken>) {
    let flib = font_lib.clone();
    let toks = tokens.clone();
    iced::application(
            move || {
                let stream = ContentStreamer::stream_content(flib.clone(), toks.clone());
                let task = Task::stream(stream);
                (Viewer { output: HashMap::new(), glyphs: vec![]}, task)
            },
            Viewer::update,
            Viewer::view
        ).run().unwrap();
}

struct Viewer {
    output: HashMap<i32, TextInfo>,
    glyphs: Vec<GlyphInfo>,
}

impl Viewer {
    fn update(&mut self, message: Message) {
        match message {
            Message::DrawText { x_pos, y_pos, str, size } => {
                let entry = self.output.entry(y_pos).or_insert(TextInfo {
                    x: x_pos,
                    txt: String::new(),
                    size: size,
                });
                entry.txt += &str;
            },
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
            output: self.output.clone(),
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
    output: HashMap<i32, TextInfo>,
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

        let page_width = 612.0;
        let page_height = 792.0;
        let mut geom: Vec<Geometry> = vec![];

        // outer rectangle
        let mut f1 = Frame::new(renderer, bounds.size());
        let outer_rect = canvas::Path::rectangle(Point { x: 0.0, y: 0.0 }, bounds.size());
        f1.fill(&outer_rect, Color::from_rgb(0.2, 0.5, 1.0));
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
            let mut txt = canvas::Text::from(
                info.str.clone()
            );
            txt.position = Point::new(self.padding_x + info.x as f32, info.y as f32 + self.padding_y);
            txt.size = info.size.into();
            frame.fill_text(txt);
            geom.push(frame.into_geometry());
        }

        return geom;
    }
}
