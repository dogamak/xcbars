use xcb::{Connection, GenericEvent};
use futures::{Async, Poll, Stream};
use error::Error;

pub struct XcbEventStream {
    conn: Connection,
}

impl XcbEventStream {
    pub fn new(conn: Connection) -> XcbEventStream {
        XcbEventStream {
            conn,
        }
    }
}

impl Stream for XcbEventStream {
    type Item = GenericEvent;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<GenericEvent>, Error> {
        match self.conn.poll_for_event() {
            Some(event) => Ok(Async::Ready(Some(event))),
            None => Ok(Async::NotReady),
        }
    }
}
