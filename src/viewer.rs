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

struct PageState {
    zoom_scale: f64,
}

impl Default for PageState {
    fn default() -> Self {
        return PageState {
            zoom_scale: 1.0,
        }
    }
}

fn colourize_pixel(alpha: u8, colour: &Option<Vec<f64>>) -> Vec<u8> {
    match colour {
        Some(vv) => {
            if vv.len() == 3 {
                // RGB
                let mut rgba = vv.iter().map(|a| (a*255.0) as u8).collect::<Vec<u8>>();
                rgba.push(alpha);
                println!("rgba {:?}", rgba);
                return rgba;
            } else if vv.len() == 1 {
                let g = (vv[0] * 255.0) as u8;
                return [g,g,g,alpha].to_vec();
            } else {
                // CMYK?
                panic!();
            }
        },
        None => {
            return [0,0,0, alpha].to_vec();
        }
    }
}

impl Page {
    fn gen_viewer_background(&self, renderer: &Renderer, bounds: iced::Rectangle) -> Geometry {
        let mut f1 = Frame::new(renderer, bounds.size());
        let outer_rect = canvas::Path::rectangle(Point { x: 0.0, y: 0.0 }, bounds.size());
        f1.fill(&outer_rect, Color::from_rgb(0.8, 0.8, 0.8));
        return f1.into_geometry();
    }

    fn rasterize_page(&self, state: &PageState) -> iced::widget::image::Handle {
        let scale_factor = self.ctx.window_scale_factor;
        println!("redrawing! {scale_factor}");

        // Number of pixels across
        let page_width = self.ctx.width;
        // Number of pixels down
        let page_height = self.ctx.height;


        let pixels_per_row = (page_width * scale_factor) as usize;
        let pixels_per_col = (page_height * scale_factor) as usize;


        // Make the page of pixels, defaulting to white, each group of 4 an RGBA channel for one pixel.
        let mut pixels = vec![255u8; pixels_per_row * pixels_per_col * 4];

        for info in self.glyphs.iter() {
            let cc = info.byte;
            let font = self.ctx.font_lib.get_font(&info.font_id);
            let glyph_id = font.ttf.lookup_glyph_index(cc as char);
            assert_ne!(glyph_id, 0);

            // Make the font in the size of that many pixels
            let (metrics, bitmap) = font.ttf.rasterize_indexed(glyph_id, info.size as f32 *scale_factor as f32);
            if metrics.width == 0 || metrics.height == 0 {
                continue;
            }
            let gap = ((info.width*scale_factor - metrics.width as f64) / 2.0).max(0.0);

            let mut y_pos = page_height * scale_factor;
            y_pos -= info.y * scale_factor;
            y_pos -= (metrics.height as i32 + metrics.ymin) as f64;

            let x_pos = info.x * scale_factor + gap;

            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let px = x_pos as i32 + col as i32;
                    let py = y_pos as i32 + row as i32;

                    if !(px >= 0 && py >= 0 && px <= pixels_per_row as i32 && py <= pixels_per_col as i32) {
                        panic!("woops? {px} {py}");
                    }

                    let alpha = bitmap[row*metrics.width + col];
                    if scale_factor == 2.0 {
                        // println!("alpha: {alpha} {:?}", info.colour);
                    }
                    let rgba = colourize_pixel(alpha, &info.colour);

                    // Each pixel is 4 times the space
                    let i = (py as usize * pixels_per_row as usize +  px as usize)*4;
                    let alpha = rgba[3] as f64 / 255.0;
                    pixels[i]   = ((rgba[0] as f64 * alpha) + (255.0 * (1.0 - alpha))) as u8;
                    pixels[i+1] = ((rgba[1] as f64 * alpha) + (255.0 * (1.0 - alpha))) as u8;
                    pixels[i+2] = ((rgba[2] as f64 * alpha) + (255.0 * (1.0 - alpha))) as u8;
                    pixels[i+3] = 255; // always fully opaque
                }
            }
        }
        let img_width = pixels_per_row as u32;
        let img_height = pixels_per_col as u32;

        let img = image::RgbaImage::from_raw(img_width, img_height, pixels).unwrap();
        let downsampled = image::imageops::resize(
            &img,
            page_width as u32,
            page_height as u32,
            image::imageops::FilterType::Lanczos3,
        );
        return iced::widget::image::Handle::from_rgba(
            page_width as u32,
            page_height as u32,
            downsampled.into_raw(),
        );
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

        let mut geom: Vec<Geometry> = vec![];

        // outer rectangle
        geom.push(self.gen_viewer_background(renderer, bounds));

        // inner rectangle
        let mut f2 = Frame::new(renderer, bounds.size());
        let inner_size = iced::Size {
            width: (page_width * state.zoom_scale) as f32,
            height: (page_height * state.zoom_scale) as f32,
        };

        let inner_rect = canvas::Path::rectangle(Point { x: (self.padding_x * state.zoom_scale) as f32, y: (self.padding_y * state.zoom_scale) as f32}, inner_size);
        f2.fill(&inner_rect, Color::from_rgb(1.0, 1.0, 1.0));
        geom.push(f2.into_geometry());

        let mut frame = Frame::new(renderer, bounds.size());
        let img = self.rasterize_page(state);

        let page_bounds = iced::Rectangle {
            x: self.padding_x as f32,
            y: self.padding_y as f32,
            width: page_width as f32,
            height: page_height as f32,
        };
        frame.draw_image(page_bounds, &img);
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
