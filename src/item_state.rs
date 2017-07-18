use pango::{self, LayoutExt, Layout};
use cairo_sys;
use pangocairo::CairoContextExt;
use bar_properties::BarProperties;
use xcb::{self, Window, Screen, Visualtype, Pixmap, Connection};
use cairo::{Context, Surface};
use std::rc::Rc;
use error::Result;

pub struct ItemState {
    bar_props: Rc<BarProperties>,
    conn: Rc<Connection>,
    content_width: u16,
    id: usize,
    pixmap: Pixmap,
    screen_number: usize,
    state: String,
    surface: Option<Surface>,
    surface_width: u16,
    visualtype: Visualtype,
    window: Window,
}

impl ItemState {
    pub fn new(
        id: usize,
        bar_props: Rc<BarProperties>,
        screen_number: usize,
        visualtype: Visualtype,
        conn: Rc<Connection>,
        window: Window,
    ) -> ItemState {
        let pixmap = conn.generate_id();
        ItemState {
            bar_props,
            conn,
            content_width: 1,
            id,
            pixmap,
            screen_number: screen_number,
            state: String::new(),
            surface: None,
            surface_width: 0,
            visualtype,
            window,
        }
    }

    #[inline]
    pub fn is_ready(&self) -> bool {
        self.surface.is_some()
    }

    #[inline]
    pub fn get_content_width(&self) -> u16 {
        self.content_width
    }

    #[inline]
    pub fn get_pixmap(&self) -> Pixmap {
        self.pixmap
    }

    #[inline]
    pub fn get_id(&self) -> usize {
        self.id
    }

    #[inline]
    fn get_screen(&self) -> Screen {
        let setup = self.conn.get_setup();
        setup.roots().nth(self.screen_number).unwrap()
    }

    fn update_surface(&mut self) -> Result<()> {
        let width = self.content_width;

        if width > self.surface_width || width < (self.surface_width / 10) * 6 ||
            self.surface.is_none()
        {
            self.surface_width = (width / 10) * 13 + 1;

            xcb::free_pixmap(&self.conn, self.pixmap);
            self.pixmap = self.conn.generate_id();

            try_xcb!(
                xcb::create_pixmap_checked,
                "failed to create pixmap",
                &self.conn,
                self.get_screen().root_depth(),
                self.pixmap,
                self.window,
                self.surface_width as u16,
                self.bar_props.area.height()
            );

            self.surface = Some(unsafe {
                Surface::from_raw_full(cairo_sys::cairo_xcb_surface_create(
                    (self.conn.get_raw_conn() as *mut cairo_sys::xcb_connection_t),
                    //self.get_screen().ptr as *mut cairo_sys::xcb_screen_t,
                    self.pixmap,
                    (&mut self.visualtype.base as *mut xcb::ffi::xcb_visualtype_t) as
                        *mut cairo_sys::xcb_visualtype_t,
                    self.surface_width as i32,
                    self.bar_props.area.height() as i32,
                ))
            });
        }
        Ok(())
    }

    fn create_layout(&self, ctx: &Context) -> Layout {
        let layout = ctx.create_pango_layout();

        layout.set_font_description(Some(&self.bar_props.font));
        layout.set_markup(self.state.as_str(), self.state.len() as i32);
        ctx.update_pango_layout(&layout);

        layout
    }

    pub fn update_size(&mut self) -> Result<()> {
        self.update_surface()?;
        let surface = match self.surface {
            Some(ref surface) => surface,
            None => return Err("no surface".into()),
        };

        let ctx = Context::new(&surface);
        let layout = self.create_layout(&ctx);

        self.content_width = layout.get_pixel_size().0 as u16;

        Ok(())
    }

    pub fn update(&mut self, update: String) -> Result<()> {
        if update != self.state {
            self.state = update;
            self.update_size()?;
            self.paint()?;
        }
        Ok(())
    }

    pub fn paint(&mut self) -> Result<()> {
        self.update_surface()?;
        let surface = match self.surface {
            Some(ref surface) => surface,
            None => return Err("no surface".into()),
        };


        let ctx = Context::new(&surface);
        let layout = self.create_layout(&ctx);

        ctx.set_source_rgb(
            self.bar_props.bg_color.red,
            self.bar_props.bg_color.green,
            self.bar_props.bg_color.blue,
        );
        ctx.paint();
        ctx.set_source_rgb(
            self.bar_props.fg_color.red,
            self.bar_props.fg_color.green,
            self.bar_props.fg_color.blue,
        );

        let text_height = self.bar_props.font.get_size() as f64 / pango::SCALE as f64;
        let baseline = self.bar_props.area.height() as f64 / 2. + (text_height / 2.) -
            (layout.get_baseline() as f64 / pango::SCALE as f64);

        ctx.move_to(0., baseline.floor() - 1.);
        ctx.update_pango_layout(&layout);
        ctx.show_pango_layout(&layout);

        self.conn.flush();

        Ok(())
    }
}
