
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct FontLib {
    pub id_to_font: HashMap<String, Font>,
}

impl FontLib {
    pub fn get_font(&self, font_id: &str) -> &Font {
        return self.id_to_font.get(font_id).unwrap();
    }
}

#[derive(Clone, Debug)]
pub struct Font {
    pub id: String,
    pub name: String,
    pub widths: Vec<u32>,
    pub first_char: u32,
    pub ttf: fontdue::Font,
    pub font_bytes: Vec<u8>,
    pub encoding: Option<String>,
}

impl Font {
    pub fn get_width(&self, c: u8) -> u32 {
        let bb = c as u32;
        let pos = bb - self.first_char;
        return self.widths[pos as usize];
    }

    pub fn char_width(&self, c: u8) -> f64 {
        let width = self.get_width(c) as f64;
        return width;
    }
}
