#[macro_export]
macro_rules! composite {
    ( $( $arg:expr ),+ ) => {
        {
            use $crate::components::Text;
            use $crate::component::{Component, ComponentCreator};
            use $crate::Error;
            use ::futures::{Poll, Async};

            struct SubComponent(Box<ComponentCreator>);

            impl<'s> From<&'s str> for SubComponent {
                fn from(s: &'s str) -> SubComponent {
                    SubComponent(Box::new(Text {
                        text: s.to_string(),
                    }))
                }
            }

            impl<C> From<C> for SubComponent
                where C: ComponentCreator + 'static,
            {
                fn from(c: C) -> SubComponent {
                    SubComponent(Box::new(c))
                }
            }

            struct CompositeComponent {
                components: Vec<SubComponent>,
                stream: CompositeComponentStream,
            }

            impl Component for CompositeComponent {
                type Error = Error;
                type Stream = CompositeComponentStream;

                fn init(&mut self) -> Result<(), Error> {
                    for component in self.components.iter_mut() {
                        component.0.init()?;
                    }
                    Ok(())
                }

                fn stream(self, handle: Handle) -> Self::Stream {
                    CompositeComponentStream {
                        states: vec![String::new(); self.components.len()],
                        streams: self.components.into_iter().enumerate()
                            .map(|(id, c)| Box::new(c.0.into_stream(handle.clone())
                                 .map(move |s| (id, s))) as Box<Stream<Item=(usize, String), Error=Error>>)
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
                    for stream in self.streams.iter_mut() {
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
                stream: CompositeComponentStream {
                    streams: vec![],
                    states: vec![],
                }
            }
        }
    };
}
