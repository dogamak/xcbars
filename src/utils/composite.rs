#[macro_export]
macro_rules! composite {
    ( $( $arg:expr ),+ ) => {
        {
            use $crate::component::{Component, SubComponent};
            use $crate::Error;
            use ::futures::{Poll, Async};

            struct CompositeComponent {
                components: Vec<SubComponent>,
            }

            impl Component for CompositeComponent {
                type Error = Error;
                type Stream = CompositeComponentStream;

                fn init(&mut self) -> Result<(), Error> {
                    for component in &mut self.components {
                        component.0.init()?;
                    }
                    Ok(())
                }

                fn stream(self, handle: Handle) -> Self::Stream {
                    CompositeComponentStream {
                        states: vec![String::new(); self.components.len()],
                        streams: self.components
                            .into_iter()
                            .enumerate()
                            .map(|(id, c)| {
                                Box::new(c.0.into_stream(handle.clone()).map(move |s| (id, s))) as
                                    Box<Stream<Item = (usize, String), Error = Error>>
                            })
                            .collect(),
                    }
                }
            }

            struct CompositeComponentStream {
                streams: Vec<Box<Stream<Item=(usize, String), Error=Error>>>,
                states: Vec<String>,
            };

            impl Stream for CompositeComponentStream {
                type Item = String;
                type Error = Error;

                fn poll(&mut self) -> Poll<Option<String>, Error> {
                    let mut do_update = false;
                    for stream in &mut self.streams {
                        match stream.poll() {
                            Ok(Async::Ready(Some((id, update)))) => {
                                do_update = true;
                                self.states[id] = update;
                            },
                            Err(err) => return Err(err),
                            _ => {},
                        }
                    }

                    match do_update {
                        true => Ok(Async::Ready(Some(self.states.concat()))),
                        false => Ok(Async::NotReady),
                    }
                }
            }

            CompositeComponent {
                components: vec![$( SubComponent::from($arg) ),+],
            }
        }
    };
}
