use std::{error, io, mem, thread};
use std::io::{ErrorKind, Write, Read};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::sync::{Arc, mpsc, Condvar};
use std::sync::Mutex;

use termio::input::{Event, EventReader};
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};
use util::io::{SafeWrite, pipeline};
use byteorder::{ReadBytesExt, WriteBytesExt, BigEndian};
use std::net::Ipv4Addr;
use std::net::Ipv6Addr;
use crate::{proxy};
use util::watch::{Watchable, Watch};
use util::{Name, lossy, cancel, expect};
use std::collections::{HashMap, HashSet};
use util::dirty::dirty_loop;
use termio::screen::Screen;
use std::thread::JoinHandle;
use crate::proxy::Host;
use std::time::Duration;
use chrono::format::Item::Error;
use util::cancel::{Cancel, Context};
use util::cancel::RecvError::Cancelling;
use std::collections::hash_map::Entry;
use std::str;
use termio::gui::gui::Gui;
use termio::gui::event::{EventSender, Priority, GuiEvent, SharedGuiEvent, EventMutex, read_loop};
use termio::gui::event;
use util::any::Upcast;
use std::any::Any;
use std::fmt::Debug;
use termio::gui::tree::Tree;
use util::mutrc::MutRc;

pub trait Model: 'static + Send + Sync + Debug + Upcast<dyn Any> {
    fn make_gui(&mut self, username: &Name, tree: Tree) -> MutRc<Gui>;
}

#[derive(Debug)]
pub struct NetcatState {
    peers: HashMap<Name, Peer>,
    streams: HashSet<Shared<TcpStream>>,
}

#[derive(Debug)]
struct Peer {
    stream: Shared<TcpStream>,
    terminate: Arc<Condvar>,
}

pub struct NetcatServer {
    listener: TcpListener,
    event_sender: EventSender,
    event_mutex: EventMutex,
    state: MutRc<NetcatState>,
    model: MutRc<dyn Model>,
}

impl NetcatServer {
    pub fn new(
        address: &str,
        event_mutex: EventMutex,
        event_sender: EventSender,
        model: MutRc<dyn Model>) -> io::Result<Arc<Self>> {
        let state = MutRc::new(NetcatState {
            peers: HashMap::new(),
            streams: HashSet::new(),
        });
        let result = Arc::new(NetcatServer {
            listener: TcpListener::bind(address)?,
            event_sender,
            event_mutex,
            state,
            model,
        });
        Ok(result)
    }
}

impl NetcatServer {
    fn handle_stream(self: &Arc<Self>,
                     mut stream: Shared<TcpStream>)
                     -> Result<(), Box<dyn error::Error>> {
        let (host, _) = proxy::run_proxy_server(&mut stream)?;
        let username = Arc::new(host.to_string()?);
        let mut lock = self.event_mutex.lock().unwrap();
        let (tree, render_loop) = event::render_loop(self.event_sender.clone());
        loop {
            let condvar = match self.state.borrow_mut().peers.entry(username.clone()) {
                Entry::Occupied(peer) => {
                    peer.get().stream.shutdown(Shutdown::Read).ok();
                    peer.get().terminate.clone()
                }
                Entry::Vacant(x) => {
                    x.insert(Peer {
                        stream: stream.clone(),
                        terminate: Arc::new(Condvar::new()),
                    });
                    break;
                }
            };
            lock = condvar.wait(lock).unwrap();
        }
        let gui = self.model.borrow_mut().make_gui(&username, tree);
        mem::drop(lock);
        let send_joiner = thread::spawn({
            let event_mutex = self.event_mutex.clone();
            let gui = gui.clone();
            let stream = stream.clone();
            move || {
                render_loop.run(event_mutex, gui, stream.clone());
                stream.shutdown(Shutdown::Both).ok();
            }
        });
        {
            if let Err(error) = read_loop(self.event_sender.clone(), gui.clone(), stream) {
                if error.kind() == ErrorKind::UnexpectedEof {
                    println!("Peer {:?} shutdown", username);
                } else {
                    println!("Peer {:?} failed: {:?}", username, error);
                }
            }
        }
        {
            let lock = self.event_mutex.lock().unwrap();
            gui.borrow_mut().set_enabled(false);
            mem::drop(lock);
        }
        send_joiner.join().unwrap();
        {
            let lock = self.event_mutex.lock().unwrap();
            self.state.borrow_mut().peers.remove(&username).unwrap().terminate.notify_all();
            mem::drop(lock);
        }
        Ok(())
    }

    pub fn listen(self: &Arc<Self>, context: Context) -> io::Result<()> {
        let self2 = self.clone();
        let on_cancel = context.on_cancel(move || {
            let lock = self2.event_mutex.lock().unwrap();
            let mut state = self2.state.borrow_mut();
            self2.listener.set_nonblocking(true).unwrap();
            for peer in state.peers.values_mut() {
                peer.stream.shutdown(Shutdown::Read).ok();
            }
            mem::drop(lock);
            TcpStream::connect(self2.listener.local_addr().unwrap()).ok();
        });
        for stream_result in self.listener.incoming() {
            if context.check().is_err() {
                return Ok(());
            }
            let stream = Shared::new(stream_result?);
            let lock = self.event_mutex.lock().unwrap();
            self.state.borrow_mut().streams.insert(stream.clone());
            mem::drop(lock);
            let self2 = self.clone();
            context.spawn(move || -> cancel::Result<()>{
                if let Err(e) = self2.handle_stream(stream.clone()) {
                    if let Some(io) = e.downcast_ref::<io::Error>() {
                        match io.kind() {
                            ErrorKind::UnexpectedEof => {}
                            ErrorKind::Interrupted => return Err(Cancel),
                            _ => eprintln!("IO error: {:?}", e),
                        }
                    } else {
                        eprintln!("Comm error: {:?}", e);
                    }
                }
                let lock = self2.event_mutex.lock().unwrap();
                self2.state.borrow_mut().streams.remove(&stream);
                mem::drop(lock);
                Ok(())
            });
        }
        mem::drop(on_cancel);
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use std::sync::{mpsc, Arc, Mutex};
    use util::{Name, expect, cancel};
    use crate::{Handler, proxy, Renderer};
    use crate::tcp::NetcatServer;
    use std::net::{TcpStream, Shutdown};
    use std::{thread, mem};
    use crate::proxy::Host;
    use std::time::Duration;
    use util::cancel::RecvError::Cancelling;
    use termio::input::Event;
    use std::io::Read;
    use std::thread::JoinHandle;
    use std::any::Any;

    struct TestHandler { log: expect::Client<Log, ()> }

    impl Handler for TestHandler {
        fn peer_add(&mut self, username: &Name) {
            self.log.execute(Log::Add(username.clone()));
        }
        fn peer_shutdown(&mut self, username: &Name) {
            self.log.execute(Log::Shutdown(username.clone()));
        }
        fn peer_close(&mut self, username: &Name) {
            self.log.execute(Log::Close(username.clone()));
        }
        fn peer_event(&mut self, username: &Name, event: &Event) {
            self.log.execute(Log::Event(username.clone(), event.clone()));
        }
        fn peer_render(&mut self, username: &Name, output: &mut Vec<u8>) {
            output.extend_from_slice(b"TEST");
            self.log.execute(Log::Render(username.clone()));
        }
    }

    fn expect_output(mut stream: TcpStream, expected: &[u8]) {
        let mut actual = vec![];
        stream.read_to_end(&mut actual).unwrap();
        assert_eq!(actual, expected);
    }

    #[cfg(test)]
    fn test() {
        let address = "127.0.0.1:9543";
        let (context, canceller, finish) = cancel::channel();
        let server = NetcatServer::new(address).unwrap();

        let (client, calls) = expect::channel();
        let handler = Arc::new(Mutex::new(TestHandler { log: client }));
        let listener = thread::spawn({
            let server = server.clone();
            move || {
                server.listen(context, handler).unwrap();
            }
        });
        let alpha = Arc::new("alpha".to_string());
        let alpha_stream = TcpStream::connect(address).unwrap();
        proxy::run_proxy_client(&alpha_stream, Host::Dns(alpha.to_string().into_bytes()), 123).unwrap();
        calls.expect(Log::Add(alpha.clone()));
        server.peer_render(&alpha);
        calls.expect(Log::Render(alpha.clone()));
        server.peer_shutdown(&alpha);
        calls.expect_and(Log::Shutdown(alpha.clone()), || server.peer_render(&alpha));

        calls.expect(Log::Render(alpha.clone()));
        calls.expect(Log::Close(alpha.clone()));
        expect_output(alpha_stream, b"TESTTEST");

        let beta = Arc::new("beta".to_string());
        let beta_stream = TcpStream::connect(address).unwrap();
        proxy::run_proxy_client(&beta_stream, Host::Dns(beta.to_string().into_bytes()), 123).unwrap();
        calls.expect(Log::Add(beta.clone()));
        beta_stream.shutdown(Shutdown::Both).unwrap();
        calls.expect_and(Log::Shutdown(beta.clone()), || server.peer_render(&beta));
        calls.expect(Log::Render(beta.clone()));
        calls.expect(Log::Close(beta.clone()));
        expect_output(beta_stream, b"");

        let gamma = Arc::new("gamma".to_string());
        let gamma_stream = TcpStream::connect(address).unwrap();
        proxy::run_proxy_client(&gamma_stream, Host::Dns(gamma.to_string().into_bytes()), 123).unwrap();
        calls.expect(Log::Add(gamma.clone()));

        let delta_stream = TcpStream::connect(address).unwrap();
        thread::sleep(Duration::from_millis(100));

        canceller.cancel();
        calls.expect_and(Log::Shutdown(gamma.clone()), || server.peer_render(&gamma));
        calls.expect(Log::Render(gamma.clone()));
        calls.expect(Log::Close(gamma.clone()));
        expect_output(gamma_stream, b"TEST");
        expect_output(delta_stream, b"");

        listener.join().unwrap();
        mem::drop(server);
        match finish.recv_timeout(Duration::from_secs(1)) {
            Err(Cancelling(joiner)) =>
                joiner.join_timeout(Duration::from_secs(1)).unwrap(),
            _ => {}
        }
    }

    #[test]
    fn test_many() {
        for _ in 0..100 {
            test();
        }
    }

    #[test]
    fn test_reconnect() {
        let address: &'static str = "127.0.0.1:9544";
        let (context, canceller, finish) = cancel::channel();
        let server = NetcatServer::new(address).unwrap();

        let (client, calls) = expect::channel();
        let handler = Arc::new(Mutex::new(TestHandler { log: client }));
        let listener = thread::spawn({
            let server = server.clone();
            move || {
                server.listen(context, handler).unwrap();
            }
        });
        let alpha = Arc::new("alpha".to_string());
        let count = 100;
        let joins: Vec<JoinHandle<TcpStream>> = (0..count).map(|_|
            thread::spawn({
                let alpha = alpha.clone();
                move || {
                    let alpha_stream = TcpStream::connect(address).unwrap();
                    proxy::run_proxy_client(&alpha_stream, Host::Dns(alpha.to_string().into_bytes()), 123).unwrap();
                    alpha_stream
                }
            })
        ).collect();
        for _ in 0..(count - 1) {
            calls.expect(Log::Add(alpha.clone()));
            calls.expect_and(Log::Shutdown(alpha.clone()), || server.peer_render(&alpha));
            calls.expect(Log::Render(alpha.clone()));
            calls.expect(Log::Close(alpha.clone()));
        }
        calls.expect(Log::Add(alpha.clone()));
        server.peer_render(&alpha);
        calls.expect(Log::Render(alpha.clone()));
        calls.expect_timeout();
        let clients: Vec<TcpStream> = joins.into_iter().map(|x| x.join().unwrap()).collect();
        let mut pending = vec![];
        for mut client in clients {
            client.set_nonblocking(true).unwrap();
            let mut actual = vec![];
            match client.read_to_end(&mut actual) {
                Ok(_) => assert_eq!(actual, b"TEST"),
                Err(_) => {
                    assert_eq!(actual, b"TEST");
                    pending.push(client)
                }
            }
        }
        canceller.cancel();
        assert_eq!(pending.len(), 1);
        let pending = pending.into_iter().next().unwrap();
        pending.set_nonblocking(false).unwrap();
        calls.expect_and(Log::Shutdown(alpha.clone()), || server.peer_render(&alpha));
        calls.expect(Log::Render(alpha.clone()));
        calls.expect(Log::Close(alpha.clone()));
        expect_output(pending, b"TEST");
        listener.join().unwrap();
        mem::drop(server);
        match finish.recv_timeout(Duration::from_secs(1)) {
            Err(Cancelling(joiner)) =>
                joiner.join_timeout(Duration::from_secs(1)).unwrap(),
            _ => {}
        }
    }
}
