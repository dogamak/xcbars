use string_component::StringComponent;
use bar::BarInfo;
use std::rc::Rc;
use tokio_core::reactor::Handle;
use std::io::BufReader;
use std::process::{Stdio, Command};
use tokio_process::{ChildStdout, CommandExt};
use futures::{Stream, Future, Poll, Async};
use tokio_timer::{Sleep, Timer};
use std::time::Duration;
use utils::LoopFn;
use error::{Result, Error};
use tokio_io::io::Lines;

pub struct Pipe {
    pub command: String,
    pub args: Vec<String>,
    pub refresh_rate: Option<Duration>,
}

impl Default for Pipe {
    fn default() -> Pipe {
        Pipe {
            command: "true".to_string(),
            args: vec![],
            refresh_rate: None,
        }
    }
}

enum PipeStreamState {
    Command(Lines<BufReader<ChildStdout>>),
    Sleep(Sleep),
}

struct PipeStream {
    handle: Handle,
    config: Pipe,
    state: PipeStreamState,
    timer: Timer,
}

fn command_lines(config: &Pipe, handle: &Handle) -> Lines<BufReader<ChildStdout>> {
    let mut cmd = Command::new(config.command.as_str());
    cmd.args(config.args.as_slice());
    cmd.stdin(Stdio::inherit()).stdout(Stdio::piped());
    let mut child = cmd.spawn_async(&handle).unwrap();
    let stdout = child.stdout().take().unwrap();
    handle.spawn(child.map(|_| ()).map_err(|_| ()));
    ::tokio_io::io::lines(BufReader::new(stdout))
}

impl PipeStream {
    fn new<'a>(config: Pipe, handle: Handle) -> PipeStream {
        PipeStream {
            state: PipeStreamState::Command(command_lines(&config, &handle)),
            handle: handle,
            config: config,
            timer: Default::default(),
        }
    }
}

impl Stream for PipeStream {
    type Item = String;
    type Error = Error;

    fn poll(&mut self) -> Poll<Option<String>, Error> {
        let mut line_received = None;
        let mut command_done = false;
        let mut sleep_done = false;
        
        match self.state {
            PipeStreamState::Command(ref mut lines) => {
                match lines.poll() {
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e.into()),
                    Ok(Async::Ready(Some(line))) => {
                        line_received = Some(line);
                    },
                    Ok(Async::Ready(None)) => {
                        command_done = true;
                    },
                }
            },
            PipeStreamState::Sleep(ref mut future) => {
                match future.poll() {
                    Ok(Async::NotReady) => return Ok(Async::NotReady),
                    Err(e) => return Err(e.into()),
                    Ok(Async::Ready(_)) => {
                        sleep_done = true;
                    },
                }
            },
        }

        if command_done {
            if let Some(duration) = self.config.refresh_rate {
                self.state = PipeStreamState::Sleep(self.timer.sleep(duration));
                return self.poll();
            } else {
                return Ok(Async::Ready(None));
            }
        }

        if let Some(line) = line_received {
            if let Some(duration) = self.config.refresh_rate {
                self.state = PipeStreamState::Sleep(self.timer.sleep(duration));
            }

            return Ok(Async::Ready(Some(line)));
        }

        if sleep_done {
            self.state = PipeStreamState::Command(command_lines(&self.config, &self.handle));
            return self.poll();
        }

        unreachable!();
    }
}

impl StringComponent for Pipe {
    type Error = Error;
    type Stream = PipeStream;

    fn create(config: Self, _: Rc<BarInfo>, handle: &Handle) -> Result<Self::Stream> {
        Ok(PipeStream::new(config, handle.clone()))
    }
}
