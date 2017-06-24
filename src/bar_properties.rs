use bar_builder::Geometry;
use pango::FontDescription;
use xcb::Rectangle;
use bar_builder::Color;

#[derive(Clone)]
pub struct BarProperties {
    pub geometry: Geometry,
    pub area: Rectangle,
    pub font: FontDescription,
    pub fg_color: Color,
    pub bg_color: Color,
    pub accent_color: Option<Color>,
}

impl BarProperties {
    #[inline]
    pub fn get_accent_color(&self) -> &Color {
        match self.accent_color {
            Some(ref color) => color,
            None => &self.fg_color,
        }
    }
}
