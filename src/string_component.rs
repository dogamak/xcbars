use futures::Stream;
use std::rc::Rc;
use bar::BarInfo;
use cairo::Surface;
use pango::{self, Layout, LayoutExt, FontMapExt};
use pangocairo::CairoContextExt;
use cairo::Context;
use tokio_core::reactor::Handle;
use component::{ComponentConfig, ComponentState};
use std::error::Error as StdError;
use std::marker::PhantomData;
use pangocairo::FontMap;

pub trait StringComponentConfig: Sized {
    type State: StringComponentState<Config = Self>;
}

pub trait StringComponentState: Sized {
    type Config: StringComponentConfig;
    type Update;
    type Error: StdError;
    type Stream: Stream<Item = Self::Update, Error = Self::Error>;

    fn create(
        config: Self::Config,
        bar_info: Rc<BarInfo>,
        handle: &Handle,
    ) -> Result<(Self, Self::Stream), Self::Error>;

    fn update(&mut self, update: <Self::Stream as Stream>::Item) -> Result<Option<String>, Self::Error>;
}

pub struct StringComponentStateInfo<S> {
    state: S,
    current: String,
    bar_info: Rc<BarInfo>,
}

impl<C> ComponentConfig for C
where
    C: StringComponentConfig,
{
    type State = StringComponentStateInfo<C::State>;
}

impl<S> StringComponentStateInfo<S> {
    fn setup_layout(&self, layout: &Layout) {
        layout.set_font_description(Some(&self.bar_info.font));
        layout.set_text(self.current.as_str());
    }
}

impl<S> ComponentState for StringComponentStateInfo<S>
where
    S: StringComponentState,
{
    type Config = S::Config;
    type Error = S::Error;
    type Update = S::Update;
    type Stream = S::Stream;

    fn create(
        config: S::Config,
        bar_info: Rc<BarInfo>,
        handle: &Handle,
    ) -> Result<(Self, Self::Stream), Self::Error> {
        let result = S::create(config, bar_info.clone(), handle);

        let (state, stream) = match result {
            Ok(x) => x,
            Err(e) => return Err(e),
        };

        let state_info = StringComponentStateInfo {
            state: state,
            current: String::new(),
            bar_info: bar_info,
        };

        Ok((state_info, stream))
    }

    fn update(&mut self, update: S::Update) -> Result<bool, Self::Error> {
        match self.state.update(update)? {
            Some(new) => {
                self.current = new;
                Ok(true)
            },
            None => Ok(false),
        }
    }

    fn render(&self, surface: &mut Surface, _: u16, height: u16) -> Result<(), Self::Error> {
        let ctx = Context::new(surface);

        let layout = ctx.create_pango_layout();
        self.setup_layout(&layout);
        ctx.update_pango_layout(&layout);

        ctx.set_source_rgb(
            self.bar_info.bg.red,
            self.bar_info.bg.green,
            self.bar_info.bg.blue,
        );
        ctx.paint();
        ctx.set_source_rgb(
            self.bar_info.fg.red,
            self.bar_info.fg.green,
            self.bar_info.fg.blue,
        );

        let text_height = self.bar_info.font.get_size() as f64 / pango::SCALE as f64;
        let baseline = height as f64 / 2. + (text_height / 2.) -
            (layout.get_baseline() as f64 / pango::SCALE as f64);

        ctx.move_to(0., baseline.floor() - 1.);
        ctx.update_pango_layout(&layout);
        ctx.show_pango_layout(&layout);

        Ok(())
    }

    fn get_preferred_width(&self) -> u16 {
        let font_map = FontMap::new();
        let ctx = font_map.create_context().unwrap();
        let layout = Layout::new(&ctx);
        self.setup_layout(&layout);

        layout.get_pixel_size().0 as u16
    }
}
