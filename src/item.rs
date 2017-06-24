use futures::Stream;
use bar_properties::BarProperties;
use tokio_core::reactor::Handle;

#[derive(Clone,Debug)]
pub enum Slot {
    Left,
    Center,
    Right,
}

pub trait ItemConfig: Sized {
    type Item: Item<Config=Self>;
}

pub trait ItemCreator {
    fn create_and_into_stream(self: Box<Self>, handle: Handle, bar_props: &BarProperties)
        -> Box<Stream<Item=String, Error=::error::Error>>;
}

impl<T> ItemCreator for T
    where T: ItemConfig,
          <<T as ItemConfig>::Item as Item>::Stream: Sized + 'static,
          <<T as ItemConfig>::Item as Item>::Error: 'static,
{
    fn create_and_into_stream(self: Box<Self>, handle: Handle, bar_props: &BarProperties)
        -> Box<Stream<Item=String, Error=::error::Error>>
    {
        let item = T::Item::create(bar_props, self);
        Box::new(item.updates(handle).map_err(|e| e.into()))
    }
}

pub trait Item: Sized {
    type Config: ItemConfig<Item=Self>;
    type Stream: Stream<Item=String, Error=Self::Error>;
    type Error: Into<::error::Error>;
    fn create(bp: &BarProperties, ic: Box<Self::Config>) -> Self;
    fn updates(self, handle: Handle) -> Self::Stream;
}
