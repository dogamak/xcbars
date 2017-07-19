use std::time::{Duration, Instant};
use tokio_core::reactor::Handle;
use error::{Error, Result};
use component::Component;
use tokio_timer::Timer;
use futures::Stream;
use time;

/// A `Clock` that automatically determines the refresh rate.
///
/// This uses the default [strftime](http://man7.org/linux/man-pages/man3/strftime.3.html)
/// syntax for formatting.
pub struct Clock {
    format: String,
    refresh_rate: Duration,
}

impl Default for Clock {
    /// Creates a `Clock` without any arguments.
    ///
    /// This uses `%T` as the default time format.
    /// [Read more](https://doc.rust-lang.org/core/default/trait.Default.html#tymethod.default)
    fn default() -> Clock {
        Clock {
            format: "%T".into(),
            refresh_rate: Duration::from_secs(1),
        }
    }
}

impl Clock {
    /// Create a new `Clock` specifying the time format as argument.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the specified `strftime` format could not be parsed.
    pub fn new<T: Into<String>>(format: T) -> Result<Clock> {
        let format = format.into();
        let refresh_rate = get_refresh_rate(&format)?;
        Ok(Clock {
            format: format,
            refresh_rate,
        })
    }
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

impl Component for Clock {
    type Error = Error;
    type Stream = Box<Stream<Item = String, Error = Error>>;

    fn stream(self, _: Handle) -> Self::Stream {
        let timer = Timer::default();

        let format = self.format.clone();
        timer.interval_at(Instant::now(), self.refresh_rate).and_then(move |()| {
            Ok(time::strftime(&format, &time::now()).unwrap())
        }).map_err(|_| "timer error".into()).boxed()
    }
}
