use xcb::Pixmap;
use cairo::Surface;
use error::Result;

pub struct ComponentContext {
    position: u16,
    width: u16,
    pixmap: Pixmap,
    surface: Surface,
}

impl ComponentContext {
    pub fn new(position: u16, width: u16) -> Result<ComponentContext> {
        unimplemented!()
    }

    #[inline]
    pub fn position(&self) -> u16 {
        self.position
    }

    #[inline]
    pub fn width(&self) -> u16 {
        self.width
    }

    #[inline]
    pub fn pixmap(&self) -> Pixmap {
        self.pixmap
    }

    pub fn update(&mut self, position: u16, width: u16) -> Result<()> {
        unimplemented!()
    }
}
