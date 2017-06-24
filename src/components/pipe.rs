use component::Component;
use tokio_core::reactor::Handle;
use std::io::BufReader;
use std::process::{Stdio, Command};
use tokio_process::{ChildStdout, CommandExt};
use tokio_io::io::Lines;
use futures::Future;

pub struct Pipe<'s> {
    pub command: &'s str,
}

impl<'s> Component for Pipe<'s> {
    type Error = ::std::io::Error;
    type Stream = Lines<BufReader<ChildStdout>>;
    
    fn stream(self, handle: Handle) -> Self::Stream {
        let mut cmd = Command::new(self.command);
        cmd.stdin(Stdio::inherit())
            .stdout(Stdio::piped());
        let mut child = cmd.spawn_async(&handle).unwrap();
        let stdout = child.stdout().take().unwrap();
        let reader = BufReader::new(stdout);
        let lines = ::tokio_io::io::lines(reader);
        handle.spawn(child.map(|_| ()).map_err(|_| ()));
        lines
    }
}
