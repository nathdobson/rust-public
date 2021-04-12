use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::Debug;
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::iter::FromIterator;
use std::mem;
use std::pin::Pin;
use std::sync::{Arc, Mutex, Weak};
use std::sync::atomic::{AtomicUsize, AtomicBool};
use std::sync::atomic::Ordering::{Relaxed, SeqCst};
use std::task::{Context, Poll, Waker, Wake};
use std::future::poll_fn;
use by_address::ByAddress;
use tokio::sync::{mpsc, oneshot, Barrier};
use tokio::sync::mpsc::unbounded_channel;
use tokio::task::yield_now;
use util::mutrc::MutRc;
use std::backtrace::Backtrace;
use tokio_stream::Stream;

use crate::waker::AtomicWaker;
use crate::join::{remote, RemoteJoinHandle};
use crate::spawn::Spawn;
use crate::futureext::FutureExt;

pub trait Priority: Send + Sync + Ord + 'static + Debug + Clone {}

impl<T: Send + Sync + Ord + 'static + Debug + Clone> Priority for T {}

pub type BoxFuture = Pin<Box<dyn 'static + Send + Future<Output=()>>>;

#[derive(Debug)]
struct Task<P: Priority> {
    state: Weak<Mutex<WakeState<P>>>,
    id: usize,
    priority: P,
}

#[derive(Debug)]
struct ArcTask<P: Priority>(Arc<Task<P>>);

#[derive(Debug)]
struct WakeState<P: Priority> {
    queue: BTreeSet<ArcTask<P>>,
    waker: Option<Waker>,
}

#[derive(Clone)]
pub struct PriorityPool<P: Priority> {
    sender: mpsc::UnboundedSender<(P, usize, BoxFuture)>,
    next_id: Arc<AtomicUsize>,
}

struct PrioritySpawn<P: Priority> {
    pool: PriorityPool<P>,
    priority: P,
}


pub struct PriorityRunner<P: Priority> {
    receiver: mpsc::UnboundedReceiver<(P, usize, BoxFuture)>,
    state: Arc<Mutex<WakeState<P>>>,
    futures: HashMap<ArcTask<P>, BoxFuture>,
}

pub fn channel<P: Priority>() -> (PriorityPool<P>, PriorityRunner<P>) {
    let (sender, receiver) = unbounded_channel();
    let state = Arc::new(Mutex::new(WakeState {
        queue: BTreeSet::new(),
        waker: None,
    }));
    (PriorityPool {
        sender,
        next_id: Arc::new(AtomicUsize::new(0)),
    }, PriorityRunner {
        receiver,
        state,
        futures: Default::default(),
    })
}

impl<P: Priority> Unpin for PriorityRunner<P> {}

impl<P: Priority> Future for PriorityRunner<P> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        let mut state = this.state.lock().unwrap();
        let finished_receive;
        loop {
            match Pin::new(&mut this.receiver).poll_recv(cx) {
                Poll::Pending => {
                    finished_receive = false;
                    break;
                }
                Poll::Ready(None) => {
                    finished_receive = true;
                    break;
                }
                Poll::Ready(Some((priority, id, next))) => {
                    let task = ArcTask(Arc::new(Task {
                        state: Arc::downgrade(&this.state),
                        id,
                        priority,
                    }));
                    assert!(this.futures.insert(task.clone(), next).is_none());
                    state.queue.insert(task.clone());
                }
            }
        }
        if let Some(first) = state.queue.pop_first() {
            if state.queue.is_empty() {
                state.waker = Some(cx.waker().clone());
            } else {
                state.waker = None;
                cx.waker().wake_by_ref();
            }
            mem::drop(state);
            if let Some(fut) = this.futures.get_mut(&first) {
                let waker = first.clone().0.into();
                let mut cx2 = Context::from_waker(&waker);
                if Pin::new(fut).poll(&mut cx2).is_ready() {
                    this.futures.remove(&first);
                }
            }
        } else {
            state.waker = Some(cx.waker().clone());
        }
        if this.futures.is_empty() && finished_receive {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }
}

impl<P: Priority> PriorityPool<P> {
    pub fn spawn(&self, priority: P, fut: impl Future<Output=()> + Send + 'static) {
        self.sender.send((
            priority,
            self.next_id.fetch_add(1, Relaxed),
            Box::pin(fut))).ok();
    }
    pub fn spawn_with_handle<T: Send + 'static>(&self, priority: P, fut: impl Future<Output=T> + Send + 'static) -> RemoteJoinHandle<T> {
        let (fut, handle) = remote(fut);
        self.spawn(priority, fut);
        handle
    }
    // pub fn at_priority(&self, priority: P) -> Executor {
    //     Arc::new(PrioritySpawn { pool: self.clone(), priority })
    // }
}

impl<P: Priority> Spawn for PrioritySpawn<P> {
    type JoinHandle<T: 'static + Send> = RemoteJoinHandle<T>;
    fn spawn_with_handle<F: 'static + Send + Future>(&self, fut: F) -> Self::JoinHandle<F::Output> where F::Output: Send {
        self.pool.spawn_with_handle(self.priority.clone(), fut)
    }
    fn spawn<F: 'static + Send + Future<Output=()>>(&self, fut: F) {
        self.pool.spawn(self.priority.clone(), fut);
    }
}

impl<P: Priority> ArcTask<P> {
    fn key(&self) -> (&P, usize) {
        (&self.0.priority, self.0.id)
    }
}

impl<P: Priority> Ord for ArcTask<P> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl<P: Priority> PartialOrd for ArcTask<P> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.key().partial_cmp(&other.key())
    }
}

impl<P: Priority> Eq for ArcTask<P> {}

impl<P: Priority> PartialEq for ArcTask<P> {
    fn eq(&self, other: &Self) -> bool {
        Arc::as_ptr(&self.0) == Arc::as_ptr(&other.0)
    }
}

impl<P: Priority> Hash for ArcTask<P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.0).hash(state)
    }
}

impl<P: Priority> Clone for ArcTask<P> {
    fn clone(&self) -> Self {
        ArcTask(self.0.clone())
    }
}

impl<P: Priority> Wake for Task<P> {
    fn wake(self: Arc<Self>) {
        if let Some(state) = self.state.upgrade() {
            let mut lock = state.lock().unwrap();
            lock.queue.insert(ArcTask(self.clone()));
            if let Some(waker) = lock.waker.take() {
                waker.wake()
            }
        }
    }
}

pub fn priority_join2<'a>(
    x: impl Future<Output=()> + Send + 'static,
    y: impl Future<Output=()> + Send + 'static) -> impl Future<Output=()> {
    let (spawner, runner) = channel::<usize>();
    spawner.spawn(0, x);
    spawner.spawn(1, y);
    mem::drop(spawner);
    runner
}

impl<P: Priority> Drop for Task<P> {
    fn drop(&mut self) {}
}

impl FromIterator<BoxFuture> for PriorityRunner<usize> {
    fn from_iter<T: IntoIterator<Item=BoxFuture>>(iter: T) -> Self {
        let (spawner, runner) = channel::<usize>();
        for (i, fut) in iter.into_iter().enumerate() {
            spawner.spawn(i, fut);
        }
        runner
    }
}

#[tokio::test]
async fn test() {
    let (sender, receiver) = oneshot::channel();
    let state1 = Arc::new(AtomicBool::new(false));
    let state2 = state1.clone();
    priority_join2(async move {
        println!("A1");
        assert_eq!(Some(1), receiver.await.ok());
        println!("A2");
        state1.store(true, SeqCst);
        println!("A3");
    }, async move {
        println!("B1");
        sender.send(1).unwrap();
        println!("B2");
        yield_now().await;
        println!("B3");
        assert!(state2.load(SeqCst));
        println!("B4");
    }).await;
}

#[tokio::test]
async fn test_cascade() {
    use rand::{thread_rng, Rng};
    use rand_xorshift::XorShiftRng;
    use rand::SeedableRng;

    for seed in 1..=100 {
        const SIZE: usize = 30;
        const COUNT: usize = 30;
        let (senders, receivers): (Vec<_>, Vec<_>) =
            (0..SIZE).map(|_| async_channel::unbounded::<Box<dyn FnOnce() + 'static + Send>>()).unzip();
        let receivers = MutRc::new(receivers);
        let (spawner, runner) = channel::<usize>();
        for i in 0..SIZE {
            let mut receivers = receivers.clone();
            spawner.spawn(
                i,
                poll_fn(move |cx| {
                    let mut receivers = receivers.write();
                    match Pin::new(&mut receivers[i]).poll_next(cx) {
                        Poll::Pending => {
                            Poll::Pending
                        }
                        Poll::Ready(None) => {
                            Poll::Ready(())
                        }
                        Poll::Ready(Some(event)) => {
                            for k in 0..i {
                                match receivers[k].try_recv() {
                                    Ok(x) => panic!("Expecting error"),
                                    _ => {}
                                }
                            }
                            event();
                            cx.waker().wake_by_ref();
                            Poll::Pending
                        }
                    }
                }),
            );
        }
        mem::drop(spawner);
        let mut rng = XorShiftRng::seed_from_u64(seed);
        for i in 0..COUNT {
            let p1 = rng.gen_range(0..SIZE);
            let p2 = rng.gen_range(0..SIZE);
            let sender2 = senders[p2].clone();
            println!("A {:?} {:?} {:?}", i, p1, p2);
            senders[p1].try_send(box move || {
                println!("B {:?} {:?} {:?}", i, p1, p2);
                sender2.try_send(box move || {
                    println!("C {:?} {:?} {:?}", i, p1, p2);
                }).unwrap();
            }).ok().unwrap();
        }
        mem::drop(senders);
        runner.await;
    }
}