use string_component::StringComponent;
use futures::stream::{once, Once};
use bar::BarInfo;
use tokio_core::reactor::Handle;
use std::rc::Rc;
use error::{Result, Error};
use std::marker::PhantomData;

pub struct Text<'s>(pub &'s str);

impl<'s> StringComponent for Text<'s> {
    type Error = Error;
    type Stream = Once<String, Error>;

    fn create(
        config: Text<'s>,
        _: Rc<BarInfo>,
        _: &Handle,
    ) -> Result<(Self::Stream)> {
        Ok(once(Ok(config.0.to_string())))
    }
}
