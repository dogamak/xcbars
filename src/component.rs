use futures::Stream;
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

pub trait ComponentState: Stream + Sized {
    type Config: ComponentConfig<State=Self>;

    fn create(config: Box<Self::Config>, handle: &Handle) -> StdResult<Self, Self::Error>;
    fn get_preferred_width(&self) -> u16;
    fn update(&mut self, update: Box<Self::Item>) -> StdResult<(), Self::Error>;
    fn render(&self, surface: Surface) -> StdResult<(), Self::Error>;
}

enum ComponentWrapper<C: ComponentConfig> {
    Config(C),
    State(C::State),
    Tmp,
}

pub trait ComponentConfigExt {
    fn create(
        self: Box<Self>,
        handle: &Handle,
    ) -> Result<Box<ComponentStateExt>>;
}

impl<C> ComponentConfigExt for C
    where C: ComponentConfig,
          <C as ComponentConfig>::State: 'static,
          <<C as ComponentConfig>::State as Stream>::Error: StdError + Send + 'static,
{
    fn create(
        self: Box<Self>,
        handle: &Handle,
    ) -> Result<Box<ComponentStateExt>> {
        let state = C::State::create(self, handle)
            .map_err(|e| Error::with_chain(e, "Failed to create component"))?;
        Ok(Box::new(state))
    }
}

pub trait ComponentStateExt {
    fn get_preferred_width(&self) -> u16;
    fn update(&mut self, update: Box<Any>) -> Result<()>;
    fn render(&self, surface: Surface) -> Result<()>;
}

impl<C> ComponentStateExt for C
    where C: ComponentState,
<C as Stream>::Error: StdError + Send + 'static,
<C as Stream>::Item: 'static,
{
    fn get_preferred_width(&self) -> u16 {
        ComponentState::get_preferred_width(self)
    } 

    fn update(&mut self, update: Box<Any>) -> Result<()> {
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
}

trait IntoBoxedComponentWrapperExt {
    fn into(self) -> Box<ComponentWrapperExt>;
}

impl<C> IntoBoxedComponentWrapperExt for C
    where C: ComponentConfig + 'static,
          <<C as ComponentConfig>::State as Stream>::Error: StdError + Send,
{
    fn into(self) -> Box<ComponentWrapperExt> {
        Box::new(ComponentWrapper::Config(self))
    } 
}

pub trait ComponentWrapperExt {
    fn create(&mut self, handle: &Handle) -> Result<()>;
    fn get_preferred_width(&self) -> u16;
    fn update(&mut self, update: Box<Any>) -> Result<()>;
    fn render(&self, surface: Surface) -> Result<()>;
}

impl<C> ComponentWrapperExt for ComponentWrapper<C>
    where C: ComponentConfig,
          <<C as ComponentConfig>::State as Stream>::Error: StdError + Send + 'static,
<<C as ComponentConfig>::State as Stream>::Item: 'static,
{
    fn create(&mut self, handle: &Handle) -> Result<()> {
        let swaped = mem::replace(self, ComponentWrapper::Tmp);
        if let ComponentWrapper::Config(conf) = swaped {
            let state = C::State::create(Box::new(conf), handle)
                .map_err(|e| Error::with_chain(e, "Failed to create component"))?;
            mem::replace(self, ComponentWrapper::State(state));
        }
        Ok(())
    }

    fn update(&mut self, update: Box<Any>) -> Result<()> {
        if let ComponentWrapper::State(ref mut conf) = *self {
            if let Ok(update) = update.downcast() {
                ComponentState::update(conf, update)
                    .map_err(|e| Error::with_chain(e, "Failed to update component"))?;
            }
        }
        Ok(())
    }

    fn render(&self, surface: Surface) -> Result<()> {
        if let ComponentWrapper::State(ref conf) = *self {
            ComponentState::render(conf, surface)
                .map_err(|e| Error::with_chain(e, "Failed to render component"))?;
        }
        Ok(())
    }

    fn get_preferred_width(&self) -> u16 {
        if let ComponentWrapper::State(ref conf) = *self {
            ComponentState::get_preferred_width(conf)
        } else {
            0
        }
    }
}

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
