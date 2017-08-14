use string_component::{StringComponentConfig, StringComponentState};
use futures::stream::{once, Once};
use bar::BarInfo;
use tokio_core::reactor::Handle;
use std::rc::Rc;
use error::{Result, Error};

pub struct Text<'s>(pub &'s str);

impl<'s> StringComponentConfig for Text<'s> {
    type State = Self;
}

impl<'s> StringComponentState for Text<'s> {
    type Config = Self;
    type Error = Error;
    type Update = ();
    type Stream = Once<(), Error>;

    fn create(config: Text<'s>, _: Rc<BarInfo>, _: &Handle) -> Result<(Self, Self::Stream)> {
        Ok((config, once(Ok(()))))
    }

    fn update(&mut self, update: ()) -> Result<Option<String>> {
        Ok(Some(self.0.to_string()))
    }
}
