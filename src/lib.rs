extern crate xcb;
extern crate mio;
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
extern crate time;

#[macro_use]
mod utils;
pub mod components;
mod error;
mod bar;
mod item_state;
mod bar_builder;
mod bar_properties;
mod xcb_event_stream;
pub mod component;

pub use bar_builder::{Color, BarBuilder, Geometry, Position};
pub use bar_properties::BarProperties;
pub use component::{Slot, Component};
pub use bar::Bar;
pub use error::Error;
