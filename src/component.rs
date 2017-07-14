use futures::{Poll, Async, Stream};
use tokio_core::reactor::Handle;
use cairo::Surface;
use std::mem;
use std::error::Error as StdError;
use std::result::Result as StdResult;
use error::*;
use bar::BarInfo;
use std::any::Any;
use std::rc::Rc;
// use components::Text;

pub trait ComponentConfig: Sized {
    type State: ComponentState<Config=Self>;
}

pub trait ComponentState: Sized {
    type Config: ComponentConfig;
    type Update;
    type Error: StdError;
    type Stream: Stream<Item=Self::Update, Error=Self::Error>;

    fn create(
        config: Self::Config,
        bar_info: Rc<BarInfo>,
        handle: &Handle,
    ) -> StdResult<(Self, Self::Stream), Self::Error>;

    fn get_preferred_width(&self) -> u16;

    fn update(
        &mut self,
        update: <Self::Stream as Stream>::Item,
    ) -> StdResult<bool, Self::Error>;
    
    fn render(&self, surface: &mut Surface, width: u16, height: u16) -> StdResult<(), Self::Error>;
}

pub trait ComponentConfigExt {
    fn create(
        self,
        bar_info: Rc<BarInfo>,
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
        println!("Poll Component");
        let update = self.stream.poll();

        match update {
            Ok(Async::Ready(Some(update))) => {
                match self.state.update(update) {
                    Ok(true) => Ok(Async::Ready(Some(()))),
                    Ok(false) => Stream::poll(self),
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
    fn render(&mut self, surface: &mut Surface, width: u16, height: u16) -> Result<()>;
    fn poll(&mut self) -> Poll<Option<()>, Error>;
}

impl<S> ComponentStateWrapperExt for ComponentStateWrapper<S>
    where S: ComponentState,
          S::Error: Send + 'static,
{
    fn get_preferred_width(&self) -> u16 {
        self.state.get_preferred_width()
    }
    
    fn render(&mut self, surface: &mut Surface, width: u16, height: u16) -> Result<()> {
        self.state.render(surface, width, height)
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
        self,
        bar_info: Rc<BarInfo>,
        handle: &Handle,
    ) -> Result<Box<ComponentStateWrapperExt>> {
        let (state, stream) = C::State::create(self, bar_info, handle)
            .map_err(|e| Error::with_chain(e, "failed to create a component"))?;

        Ok(Box::new(ComponentStateWrapper {
            state: state,
            stream: stream,
        }))
    }
}

pub enum ComponentContainer<C: ComponentConfig> {
    Config(C),
    Created,
}

impl<C> ComponentContainer<C>
    where C: ComponentConfig,
{
    pub fn new(config: C) -> ComponentContainer<C> {
        ComponentContainer::Config(config)
    }
}

pub trait ComponentContainerExt {
    fn create(&mut self, bar: Rc<BarInfo>, handle: &Handle) -> Result<Box<ComponentStateWrapperExt>>;
}

impl<C> ComponentContainerExt for ComponentContainer<C>
    where C: ComponentConfig,
          C::State: 'static,
          <C::State as ComponentState>::Error: Send + 'static,
{
    fn create(&mut self, bar: Rc<BarInfo>, handle: &Handle) -> Result<Box<ComponentStateWrapperExt>> {
        let container = mem::replace(self, ComponentContainer::Created);
        match container {
            ComponentContainer::Created => Err("component already created".into()),
            ComponentContainer::Config(config) => {
                let (state, stream) = C::State::create(config, bar, handle).unwrap();
                Ok(Box::new(ComponentStateWrapper {
                    state: state,
                    stream: stream,
                }))
            }
        }
    }
}

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
