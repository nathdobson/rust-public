use std::task::Poll;

// pub struct Pending;
//
// pub struct Ready;

trait PollExt {
    type Inner;
    // fn ready(self) -> Result<Self::Inner, Pending>;
    // fn pending(self) -> Result<Ready, Self::Inner>;
    //fn join<T>(self, other:Poll<T>) -> Poll<(Self::Inner, T)>;
    //fn
}

impl<T> PollExt for Poll<T> {
    type Inner = T;
    // fn ready(self) -> Result<Self::Inner, Pending> {
    //     match self {
    //         Poll::Ready(x) => Ok(x),
    //         Poll::Pending => Err(Pending),
    //     }
    // }
    // fn pending(self) -> Result<Ready, Self::Inner> {
    //     match self {
    //         Poll::Ready(x) => Err(x),
    //         Poll::Pending => Ok(Ready),
    //     }
    // }
}