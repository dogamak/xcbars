use component::Component;
use tokio_core::reactor::Handle;
use std::time::Duration;
use error::Error;
use xcb::{self, Connection};
use xcb_event_stream;
use futures::stream::Stream;

/// This struct is used for creation and storing the refresh rate.
pub struct WindowTitle {
    pub refresh_rate: Duration,
}

// Set the default refresh rate used for this component to one second.
impl Default for WindowTitle {
    fn default() -> WindowTitle {
        WindowTitle { refresh_rate: Duration::from_secs(1) }
    }
}

// Get the title of the window that currently is focused
fn get_window_title() -> String {
    // Connect to Xorg
    let (conn, _) = Connection::connect(None).unwrap();

    // Get the window that is currently in focus
    let input_focus_cookie = xcb::get_input_focus(&conn);
    let window = input_focus_cookie.get_reply().unwrap().focus();

    // Use ewmh to poll for the _NET_WM_NAME of the focused window
    let property_cookie = xcb::get_property(
        &conn,
        false,
        window,
        xcb::ATOM_WM_NAME,
        xcb::ATOM_STRING,
        0,
        32,
    );
    let property_reply = property_cookie.get_reply().unwrap();
    let name = String::from_utf8_lossy(property_reply.value());

    name.into()
}

// Poll the window title on a timer
impl Component for WindowTitle {
    type Error = Error;
    type Stream = Box<Stream<Item = String, Error = Error>>;

    fn stream(self, _: Handle) -> Self::Stream {
        let (conn, screen_num) = Connection::connect(None).unwrap();

        {
            let setup = conn.get_setup();
            let screen = setup.roots().nth(screen_num as usize).unwrap();

            xcb::change_window_attributes(
                &conn,
                screen.root(),
                &[(xcb::CW_EVENT_MASK, xcb::EVENT_MASK_PROPERTY_CHANGE)],
            );
        }

        conn.flush();
        let stream = xcb_event_stream::XcbEventStream::new(conn)
            .and_then(move |_| Ok(get_window_title()));
        return Box::new(stream);
    }
}
