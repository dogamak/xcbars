use futures::Stream;
use std::rc::Rc;
use bar::BarInfo;
use cairo::Surface;
use pango::{self, LayoutExt};
use pangocairo::CairoContextExt;
use cairo::Context;
use tokio_core::reactor::Handle;
use component::{ComponentConfig, ComponentState};
use error::Error;
use std::error::Error as StdError;
use std::marker::PhantomData;

pub trait StringComponent: Sized {
    type Stream: Stream<Item=String, Error=Self::Error>;
    type Error: StdError;
    
    fn create(
        config: Self,
        bar_info: Rc<BarInfo>,
        handle: &Handle,
    ) -> Result<Self::Stream, Self::Error>;
}

pub struct StringComponentStateInfo<C> {
    component: PhantomData<C>,
    current: String,
    bar_info: Rc<BarInfo>,
}

impl<C> ComponentConfig for C
    where C: StringComponent
{
    type State = StringComponentStateInfo<C>;
}

impl<C> ComponentState for StringComponentStateInfo<C>
    where C: StringComponent,
{
    type Config = C;
    type Error = C::Error;
    type Update = String;
    type Stream = C::Stream;

    fn create(
        config: C,
        bar_info: Rc<BarInfo>,
        handle: &Handle,
    ) -> Result<(Self, Self::Stream), Self::Error> {
        let result = C::create(config, bar_info.clone(), handle);

        let stream = match result {
            Ok(x) => x,
            Err(e) => return Err(e),
        };

        let state_info = StringComponentStateInfo {
            component: PhantomData,
            current: String::new(),
            bar_info: bar_info,
        };

        Ok((state_info, stream))
    }

    fn update(&mut self, update: String) -> Result<bool, Self::Error> {
        if self.current != update {
            self.current = update;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    fn render(&self, surface: &mut Surface, width: u16, height: u16) -> Result<(), Self::Error> {
        let ctx = Context::new(surface);

        let layout = ctx.create_pango_layout();

        layout.set_font_description(Some(&self.bar_info.font));
        layout.set_text(self.current.as_str(), self.current.len() as i32);
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
        100
    }
}
