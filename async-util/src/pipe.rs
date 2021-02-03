use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use futures::{AsyncRead};
use futures::task::{Context, Poll};
use std::pin::Pin;
use std::{io};
use std::task::Waker;
use std::io::Write;
use util::slice::SlicePair;

struct State {
    closed: bool,
    queue: VecDeque<u8>,
    reader: Option<Waker>,
}

struct Inner {
    state: Mutex<State>,
}

pub struct PipeWrite(Arc<Inner>);

pub struct PipeRead(Arc<Inner>);

pub fn pipe() -> (PipeWrite, PipeRead) {
    let inner =
        Arc::new(
            Inner {
                state: Mutex::new(State {
                    closed: false,
                    queue: Default::default(),
                    reader: None,
                }),
            });
    (PipeWrite(inner.clone()), PipeRead(inner))
}

impl AsyncRead for PipeRead {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8])
        -> Poll<io::Result<usize>> {
        let mut lock = self.0.state.lock().unwrap();
        if lock.queue.is_empty() {
            lock.reader = Some(cx.waker().clone());
            Poll::Pending
        } else {
            let count = buf.len().min(lock.queue.len());
            SlicePair::from_deque(&lock.queue).index(..count).copy_to(&mut buf[..count]);
            lock.queue.drain(..count);
            Poll::Ready(Ok(count))
        }
    }
}

impl Unpin for PipeRead {}

impl Unpin for PipeWrite {}

impl Drop for PipeWrite {
    fn drop(&mut self) {
        let mut lock = self.0.state.lock().unwrap();
        lock.closed = true;
        lock.reader.take().map(|x| x.wake());
    }
}

impl Write for PipeWrite {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut lock = self.0.state.lock().unwrap();
        lock.queue.extend(buf.iter());
        lock.reader.take().map(|x| x.wake());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[test]
fn test_pipe() {
    use futures::task::SpawnExt;
    use futures::executor::LocalPool;
    use futures::AsyncReadExt;
    use futures::FutureExt;

    let (mut write, mut read) = pipe();
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let joiner = spawner.spawn_with_handle(async move {
        let mut buf = [0u8; 2];
        assert_eq!(1, read.read(&mut buf).await.unwrap());
        assert_eq!(buf, [b'a', 0]);
        assert_eq!(2, read.read(&mut buf).await.unwrap());
        assert_eq!(buf, [b'b', b'c']);
        assert_eq!(1, read.read(&mut buf).await.unwrap());
        assert_eq!(buf, [b'd', b'c']);
    }).unwrap();
    pool.run_until_stalled();
    write.write(b"a").unwrap();
    pool.run_until_stalled();
    write.write(b"bcd").unwrap();
    pool.run_until_stalled();
    let () = joiner.now_or_never().unwrap();
}