use futures::{Stream, Future, IntoFuture, Poll, Async};

pub struct LoopFn<C,F> {
    creator: C,
    current: F,
}

impl<C,F> LoopFn<C,F> {
    pub fn new<I>(creator: C) -> LoopFn<C,F>
        where C: Fn() -> I,
              I: IntoFuture<Future=F,Item=F::Item,Error=F::Error>,
              F: Future,
    {
        LoopFn {
            current: creator().into_future(),
            creator,
        }
    }
}

impl<C,F,I> Stream for LoopFn<C,F>
    where C: Fn() -> I,
          I: IntoFuture<Future=F,Item=F::Item,Error=F::Error>,
          F: Future,
{
    type Item = F::Item;
    type Error = F::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        match self.current.poll() {
            Ok(Async::Ready(t)) => {
                self.current = (self.creator)().into_future();
                Ok(Async::Ready(Some(t)))
            },
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
