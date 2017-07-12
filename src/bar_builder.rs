use bar::{XcbContext, Bar, BarInfo};
use std::rc::Rc;
use error::{Result, ErrorKind};
use futures::Stream;
use pango::FontDescription;
use tokio_core::reactor::{Core, Handle};
use component_context::ComponentContext;
use component::{Slot, ComponentUpdate, ComponentConfig, ComponentConfigExt, ComponentState};
use xcb::{randr, self, Visualtype, Screen, Window, Rectangle, Connection};

#[derive(Clone)]
/// Defines a color by it's red, green and blue components.
pub struct Color {
    pub red: f64,
    pub green: f64,
    pub blue: f64,
}

impl Color {
    pub fn new(red: f64, green: f64, blue: f64) -> Color {
        Color { red, green, blue }
    }

    /// Transforms this struct into a u32, which is in reality C-packed struct
    /// with the red, green, blue and alpha fields.
    /// Colors represented in this form are required by XCB.
    pub fn as_u32(&self) -> u32 {
        (((255. * self.red).round() as u32) << 16) + (((255. * self.green).round() as u32) << 8) +
            (((255. * self.blue).round() as u32) << 0)
    }
}

/// Bar position relative to the screen.
#[derive(Clone)]
pub enum Position {
    Top,
    Bottom,
}

/// Bar geometry.
#[derive(Clone)]
pub enum Geometry {
    /// Define bar's geometry with absolute coordinates.
    Absolute(Rectangle),
    /// Define bar's geometry relative to the screen.
    Relative {
        /// The bar's position relative to the screen.
        position: Position,
        /// The bar's height in pixels.
        height: u16,
        /// Space between the bar and the top or bottom side of the screen in pixels.
        padding_x: u16,
        /// Space between the bar and the left and right side of the screen in pixels.
        padding_y: u16,
    },
}

impl Default for Geometry {
    fn default() -> Geometry {
        Geometry::Relative {
            position: Position::Top,
            height: 20,
            padding_y: 0,
            padding_x: 0,
        }
    }
}

pub type UpdateStream = Box<Stream<Item = ComponentUpdate, Error = ::error::Error>>;

/// Struct implementing the builder pattern for `Bar`.
pub struct BarBuilder {
    output: String,
    geometry: Geometry,
    bg_color: Color,
    fg_color: Color,
    font_name: String,
    items: Vec<(Slot, Box<ComponentConfigExt>)>,
}

impl BarBuilder {
    /// Create a new `BarBuilder` with default properties.
    pub fn new<T: Into<String>>(output: T) -> BarBuilder {
        BarBuilder {
            output: output.into(),
            geometry: Default::default(),
            bg_color: Color::new(1., 1., 1.),
            fg_color: Color::new(0., 0., 0.),
            items: vec![],
            font_name: String::new(),
        }
    }

    /// Create a `tokio` event loop and run launch the bar on it.
    pub fn run(self) -> Result<()> {
        let mut core = Core::new()?;
        let bar = self.build(core.handle())?;
        core.run(bar)
    }

    /// Adds a component to the bar into the specified slot.
    pub fn add_component<C>(mut self, slot: Slot, component: C) -> Self
    where
        C: ComponentConfig + 'static,
        <<C as ComponentConfig>::State as ComponentState>::Error: Send,
    {
        self.items.push((slot, Box::new(component)));
        self
    }

    /// Set the bar's position and size.
    pub fn geometry(mut self, geometry: Geometry) -> Self {
        self.geometry = geometry;
        self
    }

    /// Set the default font.
    pub fn font<S: AsRef<str>>(mut self, font: S) -> Self {
        self.font_name = font.as_ref().to_string();
        self
    }

    /// Set the default foreground color. (The text color)
    pub fn foreground(mut self, color: Color) -> Self {
        self.fg_color = color;
        self
    }

    /// Set the default background color.
    pub fn background(mut self, color: Color) -> Self {
        self.bg_color = color;
        self
    }

    /// Consumes and splits self into `self.items` and `BarProperties` struct,
    /// containing everything else relevant.
    fn into_components_and_info(
        self,
        area: &Rectangle,
    ) -> (Vec<(Slot, Box<ComponentConfigExt>)>, BarInfo) {
        let props = BarInfo {
            fg: self.fg_color,
            bg: self.bg_color,
            font: FontDescription::from_string(&*self.font_name),
        };
        (self.items, props)
    }

    /// Builds and returns the bar.
    pub fn build(self, handle: Handle) -> Result<Bar> {
        let (conn, _) = Connection::connect(None)
            .map_err(|e| ErrorKind::XcbConnection(e))?;

        let geometry;
        let window;
        let visualtype;
        let foreground = conn.generate_id();

        // Scope for `screen`
        {
            let setup = conn.get_setup();
            let screen = setup.roots().next().unwrap();

            geometry = calculate_geometry(&screen, &self.geometry, &self.output, &conn)?;
            window = create_window(&conn, &screen, &geometry, self.bg_color.as_u32())?;
            visualtype = find_visualtype(&screen).unwrap();
            
            // Create xcb graphics context for drawin te background
            try_xcb!(
                xcb::create_gc_checked,
                "failed to create gcontext",
                &conn,
                foreground,
                screen.root(),
                &[
                    (xcb::GC_FOREGROUND, self.bg_color.as_u32()),
                    (xcb::GC_GRAPHICS_EXPOSURES, 0)
                ]
            );
        }

        let xcb_ctx = Rc::new(XcbContext {
            conn: conn,
            window: window,
            visualtype: visualtype,
            screen_index: 0,
        });

        let mut components = vec![];
        let mut left_component_count = 0;
        let mut center_component_count = 0;
        let mut right_component_count = 0;

        // Consumes self
        let (items, info) = self.into_components_and_info(&geometry);
        let info = Rc::new(info);

        // Initiate components and convert them into a stream of
        // updates.  The sream also carries information about the
        // source component such as the id, slot and the index of
        // the component in the said slot.
        for (slot, config) in items.into_iter() {
            match slot {
                Slot::Left => left_component_count += 1,
                Slot::Center => center_component_count += 1,
                Slot::Right => right_component_count += 1,
            }

            let context = ComponentContext::new(xcb_ctx.clone(), geometry.height());

            let state = config.create(info.clone(), &handle)?;
            
            components.push((context, state));
        }

        Ok(Bar {
            components,
            left_component_count,
            center_component_count,
            right_component_count,
            foreground,
            geometry,
            xcb_ctx,
        })
    }
}

/// Finds a visual type matching the one of the screen provided.
fn find_visualtype<'s>(screen: &Screen<'s>) -> Option<Visualtype> {
    'DEPTH: for depth in screen.allowed_depths() {
        for visual in depth.visuals() {
            if visual.visual_id() == screen.root_visual() {
                return Some(visual);
            }
        }
    }
    None
}

/// Calculates the position and size of the bar on
/// screen given the Geometry struct and screen dimensions.
fn calculate_geometry<'s>(
    screen: &Screen<'s>,
    geometry: &Geometry,
    output: &str,
    conn: &Connection,
) -> Result<Rectangle> {
    let screen_info = get_screen_info(screen, conn, output)?;
    let screen_w = screen_info.width();
    let screen_h = screen_info.height();
    let x_offset = screen_info.x();
    let y_offset = screen_info.y();

    match geometry {
        &Geometry::Absolute(ref rect) => Ok(rect.clone()),
        &Geometry::Relative {
            ref position,
            height: bar_height,
            padding_x,
            padding_y,
        } => {
            let x;
            let y;
            let width;
            let height;

            match *position {
                Position::Top => {
                    x = x_offset + padding_x as i16;
                    y = y_offset + padding_y as i16;
                    width = screen_w - 2 * padding_x;
                    height = bar_height;
                }
                Position::Bottom => {
                    x = x_offset + padding_x as i16;
                    y = y_offset + (screen_h - bar_height - padding_y) as i16;
                    width = screen_w - 2 * padding_x;
                    height = bar_height;
                }
            };

            Ok(Rectangle::new(x, y, width, height))
        }
    }
}

// Get informatio about specified output
fn get_screen_info<'s>(
    screen: &Screen<'s>,
    conn: &Connection,
    query_output_name: &str,
) -> Result<xcb::Reply<xcb::ffi::randr::xcb_randr_get_crtc_info_reply_t>> {
    // Load screen resources of the root window
    // Return result on error
    let res_cookie = randr::get_screen_resources(conn, screen.root());
    let res_reply = res_cookie
        .get_reply()
        .map_err(|_| "Unable to get screen resources")?;

    // Get all crtcs from the reply
    let crtcs = res_reply.crtcs();

    for crtc in crtcs {
        // Get info about crtc
        let crtc_info_cookie = randr::get_crtc_info(conn, *crtc, 0);
        let crtc_info_reply = crtc_info_cookie.get_reply();

        if let Ok(reply) = crtc_info_reply {
            // Skip this crtc if it has no width or output
            if reply.width() == 0 || reply.num_outputs() == 0 {
                continue;
            }

            // Get info of crtc's first output for output name
            let output = reply.outputs()[0];
            let output_info_cookie = randr::get_output_info(conn, output, 0);
            let output_info_reply = output_info_cookie.get_reply();

            // Get the name of the first output
            let mut output_name = String::new();
            if let Ok(output_info_reply) = output_info_reply {
                output_name = String::from_utf8_lossy(output_info_reply.name()).into();
            }

            // If the output name is the requested name, return the dimensions
            if output_name == query_output_name {
                return Ok(reply);
            }
        }
    }
    let error_msg = ["Unable to find output ", query_output_name].concat();
    Err(error_msg.into())
}

/// Convinience macro for setting EWHM properites.
macro_rules! set_ewhm_prop {
    ($conn:expr, $window:expr, $name:expr, @atom $value:expr) => {
        {
            let value_atom = xcb::intern_atom($conn, true, $value)
                .get_reply().unwrap().atom();
            set_ewhm_prop!($conn, $window, $name, &[value_atom]);
        }
    };
    ($conn:expr, $window:expr, $name:expr, $data:expr) => {
        {
            let type_atom = xcb::intern_atom($conn, true, $name)
                .get_reply().unwrap().atom();
            xcb::change_property(
                $conn,
                xcb::PROP_MODE_REPLACE as u8,
                $window,
                type_atom,
                xcb::ATOM_ATOM,
                32,
                $data);
        }
    };
}

/// Creates a Xorg window using XCB.
fn create_window<'s>(
    conn: &Connection,
    screen: &Screen<'s>,
    geometry: &Rectangle,
    background: u32,
) -> Result<Window> {
    let window = conn.generate_id();

    xcb::create_window(
        &conn,
        xcb::WINDOW_CLASS_COPY_FROM_PARENT as u8,
        window,
        screen.root(),
        geometry.x(),
        geometry.y(),
        geometry.width(),
        geometry.height(),
        0,
        xcb::WINDOW_CLASS_INPUT_OUTPUT as u16,
        screen.root_visual(),
        &[
            (xcb::CW_BACK_PIXEL, background), // Default background color
            (
                xcb::CW_EVENT_MASK, // What kinds of events are we
                xcb::EVENT_MASK_EXPOSURE |       //   interested in
             xcb::EVENT_MASK_KEY_PRESS | xcb::EVENT_MASK_ENTER_WINDOW,
            ),
            (xcb::CW_OVERRIDE_REDIRECT, 0),
        ],
    );

    // TODO: Struts tell the WM to reserve space for our bar. Derive these from the Geometry struct.
    let mut strut = vec![0; 12];
    strut[2] = 30;
    strut[8] = 5;
    strut[9] = 1915;

    set_ewhm_prop!(&conn, window, "_NET_WM_WINDOW_TYPE", @atom "_NET_WM_WINDOW_TYPE_DOCK");
    set_ewhm_prop!(&conn, window, "_NET_WM_STATE", @atom "_NET_WM_STATE_STICKY");
    set_ewhm_prop!(&conn, window, "_NET_WM_DESKTOP", &[-1]);
    set_ewhm_prop!(&conn, window, "_NET_WM_STRUT_PARTIAL", strut.as_slice());
    set_ewhm_prop!(&conn, window, "_NET_WM_STRUT", &strut[0..4]);
    set_ewhm_prop!(
        &conn,
        window,
        "_NET_WM_NAME",
        "xcbars".chars().collect::<Vec<_>>().as_slice()
    ); // TODO: Allow users to define custom window title for the bar.

    // Request the WM to manage our window.
    xcb::map_window(&conn, window);

    // Some WMs (such as OpenBox) require this.
    xcb::configure_window(
        &conn,
        window,
        &[
            (xcb::CONFIG_WINDOW_X as u16, geometry.x() as u32),
            (xcb::CONFIG_WINDOW_Y as u16, geometry.y() as u32),
        ],
    );

    conn.flush();

    Ok(window)
}
