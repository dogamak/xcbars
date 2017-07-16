use std::time::{Duration, Instant};
use tokio_core::reactor::Handle;
use component::Component;
use tokio_timer::Timer;
use futures::Stream;
use error::Error;
use time;

/// A `Clock` that automatically determines the refresh rate.
///
/// This uses the default [strftime](https://linux.die.net/man/3/strftime) syntax for formatting.
/// Escaping `%` using `%%` is however not supported.
pub struct Clock {
    format: String,
    refresh_rate: Duration,
}

impl Default for Clock {
    /// Creates a `Clock` without any arguments.
    ///
    /// This uses `%T` as the default time format.
    /// [Read more](https://doc.rust-lang.org/nightly/core/default/trait.Default.html#tymethod.default)
    fn default() -> Clock {
        Clock {
            format: "%T".into(),
            refresh_rate: Duration::from_secs(1),
        }
    }
}

impl Clock {
    /// Create a new `Clock` specifying the time format as argument.
    pub fn new<T: Into<String>>(format: T) -> Clock {
        let format = format.into();
        let refresh_rate = get_refresh_rate(&format);
        Clock {
            format: format,
            refresh_rate,
        }
    }
}

// Checks if seconds are part of the `strftime` format.
// If there are no seconds the refresh rate is one minute, otherwise it's one second.
fn get_refresh_rate(format: &str) -> Duration {
    for i in &["%c", "%r", "%S", "%T", "%X"] {
        if format.contains(i) {
            return Duration::from_secs(1);
        }
    }
    Duration::from_secs(60)
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
