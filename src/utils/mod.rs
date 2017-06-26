mod stream_loop_fn;
#[macro_use]
mod composite;

pub use self::stream_loop_fn::LoopFn;

macro_rules! try_xcb {
    ($func:expr, $error:expr, $($args:expr),*) => {
        $func($($args),*)
            .request_check()
            .map_err(|e| $crate::error::Error::with_chain($crate::error::Error::from(e), $error))?;
    }
}
