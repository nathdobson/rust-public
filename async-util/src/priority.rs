use std::sync::{Arc, Mutex, Weak};
use std::future::Future;
use std::task::{Context, Poll, Waker};
use std::pin::Pin;
use futures::future::{BoxFuture, FusedFuture, FutureObj, RemoteHandle};
use futures::task::{ArcWake, SpawnExt, Spawn, SpawnError};
use crate::waker::AtomicWaker;
use std::iter::FromIterator;
use futures::task::waker;
use std::mem;
use futures::executor::LocalPool;
use futures::{FutureExt, StreamExt};
use async_channel::unbounded;
use async_std::task::yield_now;
use std::collections::{BTreeMap, HashMap, BinaryHeap, HashSet, BTreeSet};
use std::cmp::{Reverse, Ordering};
use by_address::ByAddress;
use futures::future::poll_fn;
use futures::channel::mpsc;
use std::hash::{Hash, Hasher};
use futures::Stream;
use futures::stream::FusedStream;
use util::mutrc::MutRc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering::Relaxed;
use futures::channel::mpsc::TryRecvError;
use std::fmt::Debug;
use crate::Executor;

pub trait Priority: Send + Sync + Ord + 'static + Debug + Clone {}

impl<T: Send + Sync + Ord + 'static + Debug + Clone> Priority for T {}

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
    sender: mpsc::UnboundedSender<(P, usize, FutureObj<'static, ()>)>,
    next_id: Arc<AtomicUsize>,
}

struct PrioritySpawn<P: Priority> {
    pool: PriorityPool<P>,
    priority: P,
}

pub struct PriorityRunner<P: Priority> {
    receiver: mpsc::UnboundedReceiver<(P, usize, FutureObj<'static, ()>)>,
    state: Arc<Mutex<WakeState<P>>>,
    futures: HashMap<ArcTask<P>, FutureObj<'static, ()>>,
}

pub fn channel<P: Priority>() -> (PriorityPool<P>, PriorityRunner<P>) {
    let (sender, receiver) = mpsc::unbounded();
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
            if this.receiver.is_terminated() {
                finished_receive = true;
                break;
            }
            match Pin::new(&mut this.receiver).poll_next(cx) {
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
                let waker = waker(first.clone().0);
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
        self.sender.unbounded_send((
            priority,
            self.next_id.fetch_add(1, Relaxed),
            FutureObj::from(fut.boxed()))).ok();
    }
    pub fn spawn_with_handle<T: Send + 'static>(&self, priority: P, fut: impl Future<Output=T> + Send + 'static) -> RemoteHandle<T> {
        let (fut, handle) = fut.remote_handle();
        self.spawn(priority, fut);
        handle
    }
    pub fn at_priority(&self, priority: P) -> Executor {
        Arc::new(PrioritySpawn { pool: self.clone(), priority })
    }
}

impl<P: Priority> Spawn for PrioritySpawn<P> {
    fn spawn_obj(&self, future: FutureObj<'static, ()>) -> Result<(), SpawnError> {
        self.pool.spawn(self.priority.clone(), future);
        Ok(())
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

impl<P: Priority> ArcWake for Task<P> {
    fn wake_by_ref(arc_self: &Arc<Self>) {
        if let Some(state) = arc_self.state.upgrade() {
            let mut lock = state.lock().unwrap();
            lock.queue.insert(ArcTask(arc_self.clone()));
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

impl FromIterator<BoxFuture<'static, ()>> for PriorityRunner<usize> {
    fn from_iter<T: IntoIterator<Item=BoxFuture<'static, ()>>>(iter: T) -> Self {
        let (spawner, runner) = channel::<usize>();
        for (i, fut) in iter.into_iter().enumerate() {
            spawner.spawn(i, fut);
        }
        runner
    }
}

#[test]
fn test() {
    let mut pool = LocalPool::new();
    let spawner = pool.spawner();
    let handle = spawner.spawn_with_handle(async move {
        let (sender, mut receiver1) = unbounded();
        let mut receiver2 = receiver1.clone();
        priority_join2(async move {
            println!("A1");
            assert_eq!(Some(1), receiver1.next().await);
            println!("A2");
        }, async move {
            println!("B1");
            sender.try_send(1).unwrap();
            println!("B2");
            mem::drop(sender);
            println!("B3");
            yield_now().await;
            println!("B4");
            assert!(receiver2.next().await.is_none());
            println!("B5");
        }).await
    }).unwrap();
    pool.run_until_stalled();
    handle.now_or_never().unwrap();
}

#[test]
fn test_cascade() {
    use rand::{thread_rng, Rng};
    use rand_xorshift::XorShiftRng;
    use rand::SeedableRng;
    use crate::waker::test::run_local_test;
    for seed in 0..=100 {
        const SIZE: usize = 30;
        const COUNT: usize = 30;
        let (senders, receivers): (Vec<_>, Vec<_>) =
            (0..SIZE).map(|_| mpsc::unbounded::<Box<dyn FnOnce() + 'static + Send>>()).unzip();
        let receivers = MutRc::new(receivers);
        let (spawner, runner) = channel::<usize>();
        for i in 0..SIZE {
            let mut receivers = receivers.clone();
            spawner.spawn(
                i,
                poll_fn(move |cx| {
                    let mut receivers = receivers.write();
                    if receivers[i].is_terminated() {
                        Poll::Ready(())
                    } else {
                        match receivers[i].poll_next_unpin(cx) {
                            Poll::Pending => {
                                Poll::Pending
                            }
                            Poll::Ready(None) => {
                                Poll::Ready(())
                            }
                            Poll::Ready(Some(event)) => {
                                for k in 0..i {
                                    if !receivers[k].is_terminated() {
                                        match receivers[k].try_next() {
                                            Ok(Some(x)) => panic!("Expecting error"),
                                            _ => {}
                                        }
                                    }
                                }
                                event();
                                cx.waker().wake_by_ref();
                                Poll::Pending
                            }
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
            senders[p1].unbounded_send(box move || {
                println!("B {:?} {:?} {:?}", i, p1, p2);
                sender2.unbounded_send(box move || {
                    println!("C {:?} {:?} {:?}", i, p1, p2);
                }).unwrap();
            }).unwrap();
        }
        mem::drop(senders);
        run_local_test(runner);
    }
}