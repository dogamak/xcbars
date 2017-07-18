use futures::{Future, Stream};
use std::sync::Arc;
use tokio_timer::Timer;
use tokio_core::reactor::Handle;
use component::Component;
use error::{Error, Result};
use std::time::Duration;
use utils;

#[derive(Clone, PartialEq, Copy)]
pub enum Scale {
    Binary,
    Decimal,
}

impl Scale {
    fn base(&self) -> u16 {
        match *self {
            Scale::Decimal => 1000,
            Scale::Binary => 1024,
        }
    }
}

#[derive(Clone, Copy)]
pub enum Direction {
    Incoming,
    Outgoing,
}

pub struct NetworkUsage {
    pub interface: String,
    pub direction: Direction,
    pub scale: Scale,
    pub percision: u8,
    pub refresh_frequency: Duration,
    pub sample_duration: Duration,
}

impl Default for NetworkUsage {
    fn default() -> NetworkUsage {
        NetworkUsage {
            interface: "eth0".to_string(),
            direction: Direction::Incoming,
            scale: Scale::Binary,
            percision: 3,
            refresh_frequency: Duration::from_secs(10),
            sample_duration: Duration::from_secs(1),
        }
    }
}

fn get_prefix(scale: Scale, power: u8) -> &'static str {
    match (scale, power) {
        (Scale::Decimal, 0) | (Scale::Binary, 0) => "B/s",
        (Scale::Decimal, 1) => "kb/s",
        (Scale::Decimal, 2) => "Mb/s",
        (Scale::Decimal, 3) => "Gb/s",
        (Scale::Decimal, 4) => "Tb/s",
        (Scale::Binary, 1) => "KiB/s",
        (Scale::Binary, 2) => "MiB/s",
        (Scale::Binary, 3) => "GiB/s",
        (Scale::Binary, 4) => "TiB/s",
        _ => "X/s",
    }
}

fn get_number_scale(number: u64, scale: Scale) -> (f64, u8) {
    let log = (number as f64).log(scale.base() as f64);
    let wholes = log.floor();
    let over = (scale.base() as f64).powf(log - wholes);
    (over, wholes as u8)
}

fn get_bytes(interface: &str, dir: Direction) -> ::std::io::Result<Option<u64>> {
    let dev = ::procinfo::net::dev::dev()?
        .into_iter()
        .find(|dev| dev.interface == interface);

    let dev = match dev {
        Some(dev) => dev,
        None => return Ok(None),
    };

    match dir {
        Direction::Incoming => Ok(Some(dev.receive_bytes)),
        Direction::Outgoing => Ok(Some(dev.transmit_bytes)),
    }
}

impl Component for NetworkUsage {
    type Error = Error;
    type Stream = Box<Stream<Item = String, Error = Error>>;

    fn init(&mut self) -> Result<()> {
        ::procinfo::net::dev::dev()?
            .into_iter()
            .find(|dev| dev.interface == self.interface)
            .ok_or_else(|| Error::from("No such network interface"))?;
        Ok(())
    }

    fn stream(self, _: Handle) -> Self::Stream {
        let conf = Arc::new(self);

        utils::LoopFn::new(move || {
            let timer = Timer::default();
            let conf = conf.clone();

            timer.sleep(conf.refresh_frequency)
                .and_then(move |()| {
                    let conf = conf.clone();
                    let conf2 = conf.clone();

                    let first = get_bytes(conf.interface.as_str(), conf.direction)
                        .unwrap().unwrap();

                    timer.sleep(conf.sample_duration)
                        .and_then(move |()| {
                            let second = get_bytes(conf.interface.as_str(), conf.direction)
                                .unwrap().unwrap();
                            let per_second = (second-first)/conf.sample_duration.as_secs();
                            Ok(per_second)
                        })
                        .map(move |speed| {
                            let (num, power) = get_number_scale(speed, conf2.scale);
                            let x = 10f64.powi((conf2.percision-1) as i32);
                            let num = (num*x).round() / x;
                            format!("{} {}", num, get_prefix(conf2.scale, power))
                        })
                })
        }).map_err(|_| "timer error".into())
            .boxed()
    }
}
