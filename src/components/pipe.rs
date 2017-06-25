use component::Component;
use tokio_core::reactor::Handle;
use std::io::{
    BufReader,
    BufRead,
};
use std::process::{Stdio, Command};
use tokio_process::{ChildStdout, CommandExt};
use tokio_io::io::Lines;
use futures::{future, stream, Stream, Future};
use tokio_timer::Timer;
use std::time::Duration;
use utils::LoopFn;
use error::Error;

pub struct Pipe {
    pub command: String,
    pub refresh_rate: Option<Duration>,
}

impl Pipe {
    fn reader(&self, handle: &Handle)
              -> BufReader<ChildStdout>
    {
        let mut cmd = Command::new(self.command.as_str());
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::piped());
        let mut child = cmd.spawn_async(&handle).unwrap();
        let stdout = child.stdout().take().unwrap();
        handle.spawn(child.map(|_| ()).map_err(|_| ()));
        BufReader::new(stdout)
    }
}

impl Component for Pipe {
    type Error = Error;
    type Stream = Box<Stream<Item=String, Error=Error>>;
    
    fn stream(self, handle: Handle) -> Self::Stream {
        if let Some(refresh_rate) = self.refresh_rate {
            Box::new(LoopFn::new(move || {
                let timer = Timer::default();
                Ok::<_, Error>(::tokio_io::io::lines(self.reader(&handle))
                               .map(|s| Some(s))
                               .chain(timer.sleep(refresh_rate)
                                      .into_stream()
                                      .map(|_| None)
                                      .from_err())
                               .map_err(|e| Error::from(e)))
            })
                     .flatten()
                     .filter_map(|s| s))
        } else {
            Box::new(::tokio_io::io::lines(self.reader(&handle))
                     .from_err())
        }
    }
}
