use component::Component;
use tokio_core::reactor::Handle;
use futures::{Stream, Future};
use tokio_timer::Timer;
use std::time::Duration;
use utils::LoopFn;
use error::Error;
use xcb::{self, Connection};

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
        LoopFn::new(move || {
            let timer = Timer::default();
            timer
                .sleep(self.refresh_rate)
                .and_then(|_| Ok(get_window_title()))
        }).map_err(|_| "timer error".into())
            .boxed()
    }
}
