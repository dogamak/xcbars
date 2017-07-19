use xcb::{self, Connection, GenericEvent};
use tokio_core::reactor::{PollEvented, Handle};
use std::os::unix::io::{AsRawFd, RawFd};
use futures::{Async, Poll, Stream};
use error::{Result, Error};
use mio;
use std::io;

struct InnerStream(Connection);

impl AsRawFd for InnerStream {
    fn as_raw_fd(&self) -> RawFd {
        unsafe { xcb::ffi::xcb_get_file_descriptor(self.0.get_raw_conn()) }
    }
}

impl mio::Evented for InnerStream {
    fn register(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        mio::unix::EventedFd(&self.as_raw_fd()).register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &mio::Poll,
        token: mio::Token,
        interest: mio::Ready,
        opts: mio::PollOpt,
    ) -> io::Result<()> {
        mio::unix::EventedFd(&self.as_raw_fd()).reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &mio::Poll) -> io::Result<()> {
        mio::unix::EventedFd(&self.as_raw_fd()).deregister(poll)
    }
}

pub struct XcbEventStream {
    io: PollEvented<InnerStream>,
}

impl XcbEventStream {
    pub fn new(conn: Connection, handle: &Handle) -> Result<XcbEventStream> {
        Ok(XcbEventStream {
            io: PollEvented::new(InnerStream(conn), handle)?,
        })
    }
}

impl Stream for XcbEventStream {
    type Item = GenericEvent;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<GenericEvent>, Error> {
        if Async::NotReady == self.io.poll_read() {
            return Ok(Async::NotReady);
        }

        match self.io.get_ref().0.poll_for_event() {
            Some(event) => Ok(Async::Ready(Some(event))),
            None => {
                self.io.need_read();
                Ok(Async::NotReady)
            }
        }
    }
}
