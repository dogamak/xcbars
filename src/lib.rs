extern crate xcb;
extern crate pango;
extern crate cairo;
extern crate cairo_sys;
extern crate futures;
extern crate tokio_core;
extern crate tokio_io;
extern crate tokio_process;
#[macro_use]
extern crate error_chain;
extern crate pangocairo;
extern crate procinfo;
extern crate tokio_timer;

#[macro_use]
mod utils;
mod components;
mod error;
mod item;
mod bar;
mod item_state;
mod bar_builder;
mod bar_properties;
mod xcb_event_stream;
mod component;

pub use bar_builder::{Color, BarBuilder, Geometry, Position};
pub use bar_properties::BarProperties;
pub use component::{Slot, Component};
pub use bar::Bar;

#[cfg(test)]
mod tests {
    use super::{
        BarBuilder,
        Geometry,
        Position,
        Component,
        BarProperties,
        Slot,
        Color,
    };
    use tokio_core::reactor::Handle;
    use futures::{Stream, Sink, Future};
    use futures::sync::mpsc::{channel};
    use std::thread::{spawn, sleep};
    use std::time::Duration;
    use ::error::Error;
    
    use super::components::Pipe;
    use super::components::network_usage::{
        NetworkUsage,
        Direction,
        Scale,
    };

    struct Counter {
        start: i32,
        step: i32,
    }

    impl Component for Counter {
        type Error = Error;
        type Stream = Box<Stream<Item=String, Error=Error>>;

        fn stream(self, handle: Handle)
            -> Box<Stream<Item=String, Error=Error>>
        {
            let (tx, rx) = channel(1);

            let remote = handle.remote().clone();

            spawn(move || {
                let mut current = self.start;
                loop {
                    let tx = tx.clone();
                    remote.clone().spawn(move |_| {
                        tx.send(format!("Count: {}", current))
                             .map(|_| ())
                             .map_err(|_| ())
                    });
                    current += self.step;
                    sleep(Duration::from_secs(1));
                }
            });

            Box::new(rx.map_err(|()| "channel error".into()))
        }
    }

    #[test]
    fn bar() {
        let down_speed = NetworkUsage {
            interface: "wlp58s0".to_string(),
            .. Default::default()
        };

        BarBuilder::new()
            .geometry(Geometry::Relative {
                position: Position::Top,
                height: 20,
                padding_x: 5,
                padding_y: 5,
            })
            .background(Color::new(1.0, 0.5, 0.5))
            .foreground(Color::new(1., 1., 1.))
            .font("Inconsolata 14")
            .add_component(Slot::Left, Counter {
                start: 123,
                step: 2,
            })
            .add_component(Slot::Center, Pipe {
                command: "date",
            })
            .add_component(Slot::Right, composite!("Download: ", down_speed))
            .run().unwrap();
    }
}
