use std::{error, io, mem, thread};
use async_std::io::{ErrorKind, Write, Read};
use async_std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc};

use termio::input::{Event, EventReader};
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};
use util::io::{SafeWrite, pipeline};
use async_std::net::Ipv4Addr;
use async_std::net::Ipv6Addr;
use crate::{proxy};
use util::{Name, lossy, expect};
use std::collections::{HashMap, HashSet};
use util::dirty::dirty_loop;
use termio::screen::Screen;
use crate::proxy::Host;
use std::time::Duration;
use chrono::format::Item::Error;
use std::collections::hash_map::Entry;
use std::str;
use termio::gui::gui::Gui;
use termio::gui::event::{EventSender, GuiEvent, SharedGuiEvent, read_loop, GuiPriority};
use termio::gui::event;
use std::any::Any;
use std::fmt::Debug;
use termio::gui::tree::{Tree, Dirty};
use util::mutrc::MutRc;
use std::pin::Pin;
use by_address::ByAddress;
use async_util::{Executor};
use futures::join;
use async_util::cancel::{Cancel, Canceled};
use futures::task::{SpawnExt, Spawn};
use futures::channel::oneshot;
use futures::executor::ThreadPool;
use async_util::promise::Promise;
use async_util::priority::PriorityPool;
use std::future::Future;

type StreamRc = Shared<TcpStream>;

pub trait Model: 'static + Send + Sync + Debug {
    fn add_peer(&mut self, username: &Name, tree: Tree) -> MutRc<Gui>;
    fn remove_peer(&mut self, username: &Name);
}

#[derive(Debug)]
pub struct NetcatState {
    peers: HashMap<Name, Peer>,
}

#[derive(Debug)]
struct Peer {
    stream: StreamRc,
    terminated: Promise<!>,
}

pub struct NetcatServerBuilder {
    pub cancel: Cancel,
    pub event_sender: EventSender,
}

pub struct NetcatServer {
    listener: TcpListener,
    event_sender: EventSender,
    state: MutRc<NetcatState>,
    model: MutRc<dyn Model>,
    cancel: Cancel,
}

impl NetcatServerBuilder {
    pub fn new() -> (Self, impl Future<Output=()>) {
        let cancel = Cancel::new();
        let (event_sender, event_joiner) =
            event::event_loop();
        (NetcatServerBuilder {
            cancel,
            event_sender,
        }, event_joiner)
    }

    pub async fn build(self, address: &str, model: MutRc<dyn Model>)
                       -> io::Result<Arc<NetcatServer>> {
        NetcatServer::new(
            address,
            self.event_sender,
            model,
            self.cancel,
        ).await
    }

    pub async fn run(self, address: &str, model: MutRc<dyn Model>) -> io::Result<()> {
        let server = self.build(address, model).await?;
        server.listen().await?;
        Ok(())
    }

    pub async fn run_main(self, address: &str, model: MutRc<dyn Model>) -> ! {
        self.cancel.clone().run_main(Duration::from_secs(5), self.run(address, model)).await
    }
}

impl NetcatServer {
    pub async fn new(
        address: &str,
        event_sender: EventSender,
        model: MutRc<dyn Model>,
        cancel: Cancel) -> io::Result<Arc<Self>> {
        let state = MutRc::new(NetcatState {
            peers: HashMap::new(),
        });
        let result = Arc::new(NetcatServer {
            listener: TcpListener::bind(address).await?,
            event_sender,
            state,
            model,
            cancel,
        });
        Ok(result)
    }
    async fn handle_stream(self: &Arc<Self>,
                           mut stream: StreamRc)
                           -> Result<(), Box<dyn error::Error>> {
        let user_cancel = Cancel::new();
        let (host, _) = proxy::run_proxy_server(&mut stream).await?;
        let username = Arc::new(host.to_string()?);
        let (tree, paint, layout) =
            Tree::new(user_cancel.clone(), self.event_sender.clone());
        user_cancel.attach(&self.cancel,
                           &self.event_sender.spawner().at_priority(GuiPriority::Action));

        loop {
            let terminated: Promise<!> = match self.state.borrow_mut().peers.entry(username.clone()) {
                Entry::Occupied(peer) => {
                    peer.get().stream.shutdown(Shutdown::Read).ok();
                    peer.get().terminated.clone()
                }
                Entry::Vacant(x) => {
                    x.insert(Peer {
                        stream: stream.clone(),
                        terminated: Promise::new(),
                    });
                    break;
                }
            };
            terminated.join().await;
        }
        let gui = self.model.borrow_mut().add_peer(&username, tree.clone());

        let ll = layout.layout_loop(gui.clone());
        let wl = {
            let gui = gui.clone();
            let stream = stream.clone();
            async move {
                if let Err(e) = paint.render_loop(gui, Pin::new(&mut &*stream)).await {
                    eprintln!("Send failure {}", e);
                }
                stream.shutdown(Shutdown::Both).ok();
            }
        };
        let rl = {
            let cancel = self.cancel.clone();
            let event_sender = self.event_sender.clone();
            let username = username.clone();
            async move {
                match cancel.checked(read_loop(
                    event_sender,
                    gui.clone(),
                    stream.clone())).await {
                    Ok(Ok(x)) => match x {}
                    Ok(Err(error)) => {
                        if error.kind() == ErrorKind::UnexpectedEof {
                            eprintln!("Peer {:?} shutdown", username);
                        } else {
                            eprintln!("Peer {:?} failed: {}", username, error);
                        }
                    }
                    Err(_canceled) => {
                        eprintln!("Canceled peer receive {:?}", username);
                    }
                }
                gui.borrow_mut().set_enabled(false);
                gui.borrow_mut().tree().cancel().cancel();
            }
        };
        join!(self.event_sender.spawner().spawn_with_handle(GuiPriority::Read, rl),
              self.event_sender.spawner().spawn_with_handle(GuiPriority::Layout, ll),
              self.event_sender.spawner().spawn_with_handle(GuiPriority::Paint, wl));
        self.state.borrow_mut().peers.remove(&username).unwrap();
        self.model.borrow_mut().remove_peer(&username);
        Ok(())
    }

    pub async fn listen(self: &Arc<Self>) -> io::Result<()> {
        let bundle = Promise::new();
        loop {
            let stream =
                match self.cancel.checked(self.listener.accept()).await {
                    Err(Canceled) => break,
                    Ok(stream_result) => Shared::new(stream_result?.0),
                };
            let self2 = self.clone();
            self.event_sender.spawner().spawn(GuiPriority::Read, bundle.outlive(async move {
                if let Err(e) = self2.handle_stream(stream.clone()).await {
                    if let Some(io) = e.downcast_ref::<io::Error>() {
                        match io.kind() {
                            ErrorKind::UnexpectedEof => {}
                            ErrorKind::Interrupted => { eprintln!("Cancelled") }
                            _ => eprintln!("IO error: {:?}", e),
                        }
                    } else {
                        eprintln!("Comm error: {:?}", e);
                    }
                }
            }));
        }
        eprintln!("NetcatServer waiting for servlets");
        bundle.join().await;
        eprintln!("NetcatServer done waiting for servlets");
        Ok(())
    }
}

//
// #[cfg(test)]
// mod test {
//     use std::sync::{mpsc, Arc, Mutex};
//     use util::{Name, expect, cancel};
//     use crate::{Handler, proxy, Renderer};
//     use crate::tcp::NetcatServer;
//     use std::net::{TcpStream, Shutdown};
//     use std::{thread, mem};
//     use crate::proxy::Host;
//     use std::time::Duration;
//     use util::cancel::RecvError::Cancelling;
//     use termio::input::Event;
//     use std::io::Read;
//     use std::thread::JoinHandle;
//     use std::any::Any;
//
//     struct TestHandler { log: expect::Client<Log, ()> }
//
//     impl Handler for TestHandler {
//         fn peer_add(&mut self, username: &Name) {
//             self.log.execute(Log::Add(username.clone()));
//         }
//         fn peer_shutdown(&mut self, username: &Name) {
//             self.log.execute(Log::Shutdown(username.clone()));
//         }
//         fn peer_close(&mut self, username: &Name) {
//             self.log.execute(Log::Close(username.clone()));
//         }
//         fn peer_event(&mut self, username: &Name, event: &Event) {
//             self.log.execute(Log::Event(username.clone(), event.clone()));
//         }
//         fn peer_render(&mut self, username: &Name, output: &mut Vec<u8>) {
//             output.extend_from_slice(b"TEST");
//             self.log.execute(Log::Render(username.clone()));
//         }
//     }
//
//     fn expect_output(mut stream: TcpStream, expected: &[u8]) {
//         let mut actual = vec![];
//         stream.read_to_end(&mut actual).unwrap();
//         assert_eq!(actual, expected);
//     }
//
//     #[cfg(test)]
//     fn test() {
//         let address = "127.0.0.1:9543";
//         let (context, canceller, finish) = cancel::channel();
//         let server = NetcatServer::new(address).unwrap();
//
//         let (client, calls) = expect::channel();
//         let handler = Arc::new(Mutex::new(TestHandler { log: client }));
//         let listener = thread::spawn({
//             let server = server.clone();
//             move || {
//                 server.listen(context, handler).unwrap();
//             }
//         });
//         let alpha = Arc::new("alpha".to_string());
//         let alpha_stream = TcpStream::connect(address).unwrap();
//         proxy::run_proxy_client(&alpha_stream, Host::Dns(alpha.to_string().into_bytes()), 123).unwrap();
//         calls.expect(Log::Add(alpha.clone()));
//         server.peer_render(&alpha);
//         calls.expect(Log::Render(alpha.clone()));
//         server.peer_shutdown(&alpha);
//         calls.expect_and(Log::Shutdown(alpha.clone()), || server.peer_render(&alpha));
//
//         calls.expect(Log::Render(alpha.clone()));
//         calls.expect(Log::Close(alpha.clone()));
//         expect_output(alpha_stream, b"TESTTEST");
//
//         let beta = Arc::new("beta".to_string());
//         let beta_stream = TcpStream::connect(address).unwrap();
//         proxy::run_proxy_client(&beta_stream, Host::Dns(beta.to_string().into_bytes()), 123).unwrap();
//         calls.expect(Log::Add(beta.clone()));
//         beta_stream.shutdown(Shutdown::Both).unwrap();
//         calls.expect_and(Log::Shutdown(beta.clone()), || server.peer_render(&beta));
//         calls.expect(Log::Render(beta.clone()));
//         calls.expect(Log::Close(beta.clone()));
//         expect_output(beta_stream, b"");
//
//         let gamma = Arc::new("gamma".to_string());
//         let gamma_stream = TcpStream::connect(address).unwrap();
//         proxy::run_proxy_client(&gamma_stream, Host::Dns(gamma.to_string().into_bytes()), 123).unwrap();
//         calls.expect(Log::Add(gamma.clone()));
//
//         let delta_stream = TcpStream::connect(address).unwrap();
//         thread::sleep(Duration::from_millis(100));
//
//         canceller.cancel();
//         calls.expect_and(Log::Shutdown(gamma.clone()), || server.peer_render(&gamma));
//         calls.expect(Log::Render(gamma.clone()));
//         calls.expect(Log::Close(gamma.clone()));
//         expect_output(gamma_stream, b"TEST");
//         expect_output(delta_stream, b"");
//
//         listener.join().unwrap();
//         mem::drop(server);
//         match finish.recv_timeout(Duration::from_secs(1)) {
//             Err(Cancelling(joiner)) =>
//                 joiner.join_timeout(Duration::from_secs(1)).unwrap(),
//             _ => {}
//         }
//     }
//
//     #[test]
//     fn test_many() {
//         for _ in 0..100 {
//             test();
//         }
//     }
//
//     #[test]
//     fn test_reconnect() {
//         let address: &'static str = "127.0.0.1:9544";
//         let (context, canceller, finish) = cancel::channel();
//         let server = NetcatServer::new(address).unwrap();
//
//         let (client, calls) = expect::channel();
//         let handler = Arc::new(Mutex::new(TestHandler { log: client }));
//         let listener = thread::spawn({
//             let server = server.clone();
//             move || {
//                 server.listen(context, handler).unwrap();
//             }
//         });
//         let alpha = Arc::new("alpha".to_string());
//         let count = 100;
//         let joins: Vec<JoinHandle<TcpStream>> = (0..count).map(|_|
//             thread::spawn({
//                 let alpha = alpha.clone();
//                 move || {
//                     let alpha_stream = TcpStream::connect(address).unwrap();
//                     proxy::run_proxy_client(&alpha_stream, Host::Dns(alpha.to_string().into_bytes()), 123).unwrap();
//                     alpha_stream
//                 }
//             })
//         ).collect();
//         for _ in 0..(count - 1) {
//             calls.expect(Log::Add(alpha.clone()));
//             calls.expect_and(Log::Shutdown(alpha.clone()), || server.peer_render(&alpha));
//             calls.expect(Log::Render(alpha.clone()));
//             calls.expect(Log::Close(alpha.clone()));
//         }
//         calls.expect(Log::Add(alpha.clone()));
//         server.peer_render(&alpha);
//         calls.expect(Log::Render(alpha.clone()));
//         calls.expect_timeout();
//         let clients: Vec<TcpStream> = joins.into_iter().map(|x| x.join().unwrap()).collect();
//         let mut pending = vec![];
//         for mut client in clients {
//             client.set_nonblocking(true).unwrap();
//             let mut actual = vec![];
//             match client.read_to_end(&mut actual) {
//                 Ok(_) => assert_eq!(actual, b"TEST"),
//                 Err(_) => {
//                     assert_eq!(actual, b"TEST");
//                     pending.push(client)
//                 }
//             }
//         }
//         canceller.cancel();
//         assert_eq!(pending.len(), 1);
//         let pending = pending.into_iter().next().unwrap();
//         pending.set_nonblocking(false).unwrap();
//         calls.expect_and(Log::Shutdown(alpha.clone()), || server.peer_render(&alpha));
//         calls.expect(Log::Render(alpha.clone()));
//         calls.expect(Log::Close(alpha.clone()));
//         expect_output(pending, b"TEST");
//         listener.join().unwrap();
//         mem::drop(server);
//         match finish.recv_timeout(Duration::from_secs(1)) {
//             Err(Cancelling(joiner)) =>
//                 joiner.join_timeout(Duration::from_secs(1)).unwrap(),
//             _ => {}
//         }
//     }
// }
//
