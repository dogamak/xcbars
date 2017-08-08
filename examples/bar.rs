extern crate xcbars;
extern crate futures;
extern crate tokio_core;

use xcbars::{BarBuilder, BarInfo, Geometry, Color, Slot, StringComponent, Error, Position};

use xcbars::components::{Pipe, NetworkUsage};

use futures::{Sink, Stream, Future};
use tokio_core::reactor::Handle;
use std::thread::{spawn, sleep};
use std::time::Duration;
use futures::sync::mpsc::channel;
use std::rc::Rc;

struct Counter {
    start: i32,
    step: i32,
}

impl StringComponent for Counter {
    type Error = Error;
    type Stream = Box<Stream<Item = String, Error = Error>>;

    fn create(
        config: Self,
        _: Rc<BarInfo>,
        handle: &Handle,
    ) -> Result<Box<Stream<Item = String, Error = Error>>, Error> {
        let (tx, rx) = channel(1);

        let remote = handle.remote().clone();

        spawn(move || {
            let mut current = config.start;
            loop {
                let tx = tx.clone();
                remote.clone().spawn(move |_| {
                    tx.send(format!("Count: {}", current))
                        .map(|_| ())
                        .map_err(|_| ())
                });
                current += config.step;
                sleep(Duration::from_secs(1));
            }
        });

        Ok(Box::new(rx.map_err(|()| "channel error".into())))
    }
}

fn main() {
    let down_speed = NetworkUsage {
        interface: "enp0s31f6".to_string(),
        ..Default::default()
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
        .add_component(
            Slot::Left,
            Counter {
                start: 123,
                step: 2,
            },
        )
        .add_component(
            Slot::Center,
            Pipe {
                command: "date".into(),
                args: Vec::new(),
                refresh_rate: Some(Duration::from_secs(1)),
            },
        )
        .add_component(Slot::Right, down_speed)
        .run()
        .unwrap();
}
