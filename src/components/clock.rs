use std::time::{Duration, Instant};
use tokio_core::reactor::Handle;
use error::{Error, Result};
use string_component::{StringComponentState, StringComponentConfig};
use tokio_timer::{Interval, Timer};
use futures::Stream;
use bar::BarInfo;
use std::rc::Rc;
use time;

/// A `Clock` that automatically determines the refresh rate.
///
/// This uses the default [strftime](http://man7.org/linux/man-pages/man3/strftime.3.html)
/// syntax for formatting.
pub struct Clock<'s> {
    pub format: &'s str,
}

// Checks if seconds are part of the `strftime` format.
// If there are no seconds the refresh rate is one minute, otherwise it's one second.
fn get_refresh_rate(format: &str) -> Result<Duration> {
    // Create a time with any non-zero second value
    let mut time = time::now();
    time.tm_sec = 15;

    // Convert to String and back to Tm
    let time_str = time::strftime(format, &time)
        .map_err(|_| "Invalid clock format.")?;
    let time = time::strptime(&time_str, format)
        .map_err(|_| "Invalid clock format.")?;

    // Check if seconds are still there
    if time.tm_sec != 0 {
        Ok(Duration::from_secs(1))
    } else {
        Ok(Duration::from_secs(60))
    }
}

impl<'s> StringComponentConfig for Clock<'s> {
    type State = Self;
}

impl<'s> StringComponentState for Clock<'s> {
    type Config = Self;
    type Error = Error;
    type Update = ();
    type Stream = Box<Stream<Item=(), Error=Error>>;

    fn create(config: Self, _: Rc<BarInfo>, _: &Handle) -> Result<(Self, Self::Stream)> {
        let timer = Timer::default();

        let refresh_rate = get_refresh_rate(config.format)?;
        let stream = timer.interval_at(Instant::now(), refresh_rate)
            .map_err(|_| "timer error".into());

        // Check that the format can be parsed
        if let Err(_) = time::strftime(&config.format, &time::now()) {
            return Err("time format parsing error".into());
        }

        Ok((config, Box::new(stream)))
    }

    fn update(&mut self, _: ()) -> Result<Option<String>> {
        Ok(Some(time::strftime(&self.format, &time::now()).unwrap()))
    }
}
