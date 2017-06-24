use component::Component;
use futures::stream::{once, Once};
use tokio_core::reactor::Handle;
use error::Error;

pub struct Text {
    pub text: String,
}

impl Component for Text {
    type Error = Error;
    type Stream = Once<String, Error>;

    fn stream(self, _: Handle) -> Self::Stream {
        once(Ok(self.text))
    }
}
