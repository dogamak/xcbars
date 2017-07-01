use xcb::{self, Pixmap, Connection, Window, Screen};
use cairo_sys;
use cairo::Surface;
use error::Result;
use std::rc::Rc;
use bar::XcbContext;

struct ComponentGraphicalContext {
    position: u16,
    width: u16,
    pixmap: Pixmap,
    surface: Surface,
    screen_index: usize,
}

pub struct ComponentContext {
    graphical_context: Option<ComponentGraphicalContext>,
    xcb_context: Rc<XcbContext>,
    height: u16,
    pixmap_width: u16,
}

impl ComponentContext {
    pub fn new(xcb_context: Rc<XcbContext>, height: u16) -> ComponentContext {
        ComponentContext {
            graphical_context: None,
            xcb_context: xcb_context,
            height: height,
            pixmap_width: 0,
        }
    }

    #[inline]
    pub fn position(&self) -> Option<u16> {
        match self.graphical_context {
            Some(ref gctx) => Some(gctx.position),
            None => None,
        }
    }

    #[inline]
    pub fn width(&self) -> Option<u16> {
        match self.graphical_context {
            Some(ref gctx) => Some(gctx.width),
            None => None,
        }
    }

    #[inline]
    pub fn pixmap(&self) -> Option<Pixmap> {
        match self.graphical_context {
            Some(ref gctx) => Some(gctx.pixmap),
            None => None,
        }
    }

    #[inline]
    pub fn surface(&mut self) -> Option<&mut Surface> {
        match self.graphical_context {
            Some(ref mut gctx) => Some(&mut gctx.surface),
            None => None,
        }
    }

    pub fn update(&mut self, position: u16, mut width: u16) -> Result<()> {
        if width > self.pixmap_width {
            width = (width*13)/10;
        } else if width > (self.pixmap_width*8)/10 {
            return Ok(());
        }

        let pixmap = self.create_pixmap(width)?;
        let surface = self.create_surface(width, pixmap);

        if let Some(ref mut gctx) = self.graphical_context {
            gctx.pixmap = pixmap;
            gctx.surface = surface;
        }
        
        self.pixmap_width = width;

        Ok(())
    }

    fn create_pixmap(&self, width: u16) -> Result<Pixmap> {
        let pixmap = self.xcb_context.conn.generate_id();
        try_xcb!(
            xcb::create_pixmap_checked,
            "failed to create pixmap",
            &self.xcb_context.conn,
            self.xcb_context.screen().root_depth(),
            pixmap,
            self.xcb_context.window,
            width,
            self.height
        );
        Ok(pixmap)
    }

    fn create_surface(&mut self, width: u16, pixmap: Pixmap) -> Surface {
        unsafe {
            Surface::from_raw_full(cairo_sys::cairo_xcb_surface_create(
                (self.xcb_context.conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t),
                //self.get_screen().ptr as *mut cairo_sys::xcb_screen_t,
                pixmap,
                (&self.xcb_context.visualtype.base as *const xcb::ffi::xcb_visualtype_t) as
                    *mut cairo_sys::xcb_visualtype_t,
                self.pixmap_width as i32,
                self.height as i32,
            ))
        }
    }
}
