use component::Component;
use tokio_core::reactor::Handle;
use error::{Error, Result, ErrorKind};
use xcb::{self, Connection};
use xcb_event_stream;
use futures::stream::Stream;

/// This struct is used for the window title component.
pub struct WindowTitle {
    active_window: xcb::AtomEnum,
    wm_name: xcb::AtomEnum,
}

impl Default for WindowTitle {
    /// Create the default `WindowTitle` component.
    /// The default by itself does not work, `init` is required.
    fn default() -> WindowTitle {
        WindowTitle {
            active_window: 0,
            wm_name: 0,
        }
    }
}

impl WindowTitle {
    // Get the title of the window that currently is focused
    fn get_window_title(&self) -> Result<String> {
        // Connect to Xorg
        let (conn, screen_num) = Connection::connect(None).map_err(ErrorKind::XcbConnection)?;

        // Get the screen for accessing the root window later
        let setup = conn.get_setup();
        let screen = setup
            .roots()
            .nth(screen_num as usize)
            .ok_or("Unable to acquire screen.")?;

        // Get the currently active window
        let property_reply = xcb::get_property(
            &conn,
            false,
            screen.root(),
            self.active_window,
            xcb::ATOM_WINDOW,
            0,
            32,
        ).get_reply()?;

        // Get the window from the reply
        // Returns with error if no window has been found
        let active_window_value: &[u32] = property_reply.value();
        if active_window_value.is_empty() {
            return Err("Could not get active window.".into());
        }
        let window = active_window_value[0];

        // Get the title of the active window
        let property_reply =
            xcb::get_property(&conn, false, window, self.wm_name, xcb::ATOM_ANY, 0, 32)
                .get_reply()?;

        // Convert active window reply to string and return it
        let name = String::from_utf8_lossy(property_reply.value());
        Ok(name.into())
    }
}

// Poll the window title on a timer
impl Component for WindowTitle {
    type Error = Error;
    type Stream = Box<Stream<Item = String, Error = Error>>;

    fn stream(self, handle: Handle) -> Self::Stream {
        let (conn, screen_num) = Connection::connect(None).unwrap();

        // Setup the event for the root screen
        {
            let setup = conn.get_setup();
            let screen = setup.roots().nth(screen_num as usize).unwrap();

            xcb::change_window_attributes(
                &conn,
                screen.root(),
                &[(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_PROPERTY_CHANGE)],
            );
        }

        // Start event receiver future
        // Executes `get_window_title` every time window title event is received
        conn.flush();
        let active_window = self.active_window;
        let stream = xcb_event_stream::XcbEventStream::new(conn, &handle)
            .unwrap()
            .filter(move |event| unsafe {
                let property_event: &xcb::PropertyNotifyEvent = xcb::cast_event(event);
                let property_atom = property_event.atom();
                property_atom == active_window
            })
            .and_then(move |_| {
                self.get_window_title().or_else(|_| Ok(String::new()))
            });

        Box::new(stream)
    }

    // Set fields of `WindowTitle` struct.
    //
    // # Errors
    //
    // If there is a problem with X or the Atoms are not supported, this will fail.
    fn init(&mut self) -> Result<()> {
        let (conn, _) = Connection::connect(None).map_err(ErrorKind::XcbConnection)?;

        let active_window = xcb::intern_atom(&conn, true, "_NET_ACTIVE_WINDOW")
            .get_reply()?
            .atom();
        let wm_name = xcb::intern_atom(&conn, true, "_NET_WM_NAME")
            .get_reply()?
            .atom();

        // Fail if one of the atoms is not supported
        if wm_name == 0 || active_window == 0 {
            return Err("The required EWMH properties are not supported.".into());
        }

        self.active_window = active_window;
        self.wm_name = wm_name;

        Ok(())
    }
}
