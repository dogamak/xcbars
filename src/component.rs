use futures::{Poll, Async, Stream};
use tokio_core::reactor::Handle;
use cairo::Surface;
use std::mem;
use std::error::Error as StdError;
use std::result::Result as StdResult;
use error::*;
use std::any::Any;
// use components::Text;

pub trait ComponentConfig: Sized {
    type State: ComponentState<Config=Self>;
}

pub trait ComponentState: Sized {
    type Config: ComponentConfig<State=Self>;
    type Update;
    type Error: StdError;
    type Stream: Stream<Item=Self::Update, Error=Self::Error>;

    fn create(
        config: Box<Self::Config>,
        handle: &Handle,
    ) -> StdResult<(Self, Self::Stream), Self::Error>;

    fn get_preferred_width(&self) -> u16;

    fn update(
        &mut self,
        update: <Self::Stream as Stream>::Item,
    ) -> StdResult<bool, Self::Error>;
    
    fn render(&self, surface: &mut Surface) -> StdResult<(), Self::Error>;
}

pub trait ComponentConfigExt {
    fn create(
        self: Box<Self>,
        handle: &Handle,
    ) -> Result<Box<ComponentStateWrapperExt>>;
}

struct ComponentStateWrapper<S: ComponentState> {
    state: S,
    stream: S::Stream,
}

impl<S> Stream for ComponentStateWrapper<S>
    where S: ComponentState,
          S::Error: Send + 'static,
{
    type Item = ();
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<()>, Error> {
        let update = self.stream.poll();

        match update {
            Ok(Async::Ready(Some(update))) => {
                match self.state.update(update) {
                    Ok(true) => Ok(Async::Ready(Some(()))),
                    Ok(false) => Ok(Async::NotReady),
                    Err(err) => Err(Error::with_chain(err, "failed to update component")),
                }
            },
            Err(err) => Err(Error::with_chain(err, "failed to check component updates")),
            Ok(Async::Ready(None)) => Ok(Async::Ready(None)),
            Ok(Async::NotReady) => Ok(Async::NotReady),
        }
    }
}

pub trait ComponentStateWrapperExt {
    fn get_preferred_width(&self) -> u16;
    fn render(&mut self, surface: &mut Surface) -> Result<()>;
    fn poll(&mut self) -> Poll<Option<()>, Error>;
}

impl<S> ComponentStateWrapperExt for ComponentStateWrapper<S>
    where S: ComponentState,
          S::Error: Send + 'static,
{
    fn get_preferred_width(&self) -> u16 {
        self.state.get_preferred_width()
    }
    
    fn render(&mut self, surface: &mut Surface) -> Result<()> {
        self.state.render(surface)
            .map_err(|e| Error::with_chain(e, "failed to render a component"))
    }

    fn poll(&mut self) -> Poll<Option<()>, Error> {
        Stream::poll(self)
    }
}

impl<C> ComponentConfigExt for C
    where C: ComponentConfig,
          <C as ComponentConfig>::State: ComponentState + 'static,
          <<C as ComponentConfig>::State as ComponentState>::Error: Send,
{
    fn create(
        self: Box<Self>,
        handle: &Handle,
    ) -> Result<Box<ComponentStateWrapperExt>> {
        let (state, stream) = C::State::create(self, handle)
            .map_err(|e| Error::with_chain(e, "failed to create a component"))?;

        Ok(Box::new(ComponentStateWrapper {
            state: state,
            stream: stream,
        }))
    }
}

/* pub trait ComponentStateExt {
    fn get_preferred_width(&self) -> u16;
    fn update(&mut self, update: Box<Any>) -> Result<bool>;
    fn render(&self, surface: Surface) -> Result<()>;
}

impl<C> ComponentStateExt for C
    where C: ComponentState,
          <<C as ComponentState>::Stream as Stream>::Error: StdError + Send + 'static,
          <<C as ComponentState>::Stream as Stream>::Item: 'static,
{
    fn get_preferred_width(&self) -> u16 {
        ComponentState::get_preferred_width(self)
    } 

    fn update(&mut self, update: Box<Any>) -> Result<bool> {
        if let Ok(update) = update.downcast() {
            Ok(ComponentState::update(self, update)
               .map_err(|e| Error::with_chain(e, "Failed to update component"))?)
        } else {
            Ok(())
        }
    }

    fn render(&self, surface: Surface) -> Result<()> {
        Ok(ComponentState::render(self, surface)
           .map_err(|e| Error::with_chain(e, "Failed to render component"))?)
    }
} */

/* pub struct SubComponent(pub Box<ComponentCreator>);

impl<'s> From<&'s str> for SubComponent {
    fn from(s: &'s str) -> SubComponent {
        SubComponent(Box::new(Text { text: s.to_string() }))
    }
}

impl<C> From<C> for SubComponent
where
    C: ComponentCreator + 'static,
{
    fn from(c: C) -> SubComponent {
        SubComponent(Box::new(c))
    }
} */

#[derive(Clone, Copy, Debug)]
pub enum Slot {
    Left,
    Right,
    Center,
}

pub struct ComponentUpdate {
    pub slot: Slot,
    pub index: usize,
    pub id: usize,
    pub value: String,
}
