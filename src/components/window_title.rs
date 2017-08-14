use string_component::{StringComponentConfig, StringComponentState};
use tokio_core::reactor::Handle;
use error::{Error, Result, ErrorKind};
use xcb::{self, Connection};
use xcb_event_stream;
use futures::stream::Stream;
use std::rc::Rc;
use bar::BarInfo;
use xcb_event_stream::XcbEventStream;

pub struct WindowTitleConfig;

/// This struct is used for the window title component.
pub struct WindowTitleState {
    active_window: xcb::AtomEnum,
    wm_name: xcb::AtomEnum,
}

impl WindowTitleState {
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

impl StringComponentConfig for WindowTitleConfig {
    type State = WindowTitleState;
}

// Poll the window title on a timer
impl StringComponentState for WindowTitleState {
    type Error = Error;
    type Update = xcb::GenericEvent;
    type Stream = XcbEventStream;
    type Config = WindowTitleConfig;

    fn create(_: WindowTitleConfig, _: Rc<BarInfo>, handle: &Handle) -> Result<(Self, Self::Stream)> {
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
        
        let state = WindowTitleState {
            active_window: active_window,
            wm_name: wm_name,
        };
        
        // Start event receiver future
        // Executes `get_window_title` every time window title event is received
        conn.flush();
        let active_window = state.active_window;
        let stream = xcb_event_stream::XcbEventStream::new(conn, &handle)?;

        Ok((state, stream))
    }

    fn update(&mut self, event: xcb::GenericEvent) -> Result<Option<String>> {
        let property_event: &xcb::PropertyNotifyEvent = unsafe {xcb::cast_event(&event) };
        let property_atom = property_event.atom();
        
        if property_atom == self.active_window {
            Ok(Some(self.get_window_title().unwrap_or(String::new())))
        } else {
            Ok(None)
        }
    }
}
