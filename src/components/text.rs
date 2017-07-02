use string_component::StringComponent;
use futures::stream::{once, Once};
use tokio_core::reactor::Handle;
use error::Error;

pub struct Text {
    pub text: String,
}

impl StringComponent for Text {
    type Error = Error;
    type Stream = Once<String, Error>;

    fn into_stream(self: Box<Text>, _: &Handle) -> Self::Stream {
        once(Ok(self.text))
    }
}
