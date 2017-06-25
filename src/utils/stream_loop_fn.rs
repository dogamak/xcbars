use futures::{Stream, Future, IntoFuture, Poll, Async};

pub struct LoopFn<C,F> {
    creator: C,
    current: Option<F>,
}

impl<C,F> LoopFn<C,F> {
    pub fn new<I>(creator: C) -> LoopFn<C,F>
        where C: Fn() -> I,
              I: IntoFuture<Future=F,Item=F::Item,Error=F::Error>,
              F: Future,
    {
        LoopFn {
            creator,
            current: None,
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
        if self.current.is_none() {
            self.current = Some((self.creator)().into_future());
        }

        let current;
        if let Some(ref mut future) = self.current {
            current = future.poll();
        } else {
            unreachable!();
        }

        match current {
            Ok(Async::Ready(t)) => {
                self.current = Some((self.creator)().into_future());
                Ok(Async::Ready(Some(t)))
            },
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Err(e) => Err(e),
        }
    }
}
