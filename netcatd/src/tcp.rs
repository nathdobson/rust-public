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
use termio::gui::node::{Node, NodeStrong};
use termio::gui::controller::Controller;
use termio::gui::view::ViewImpl;
use termio::gui::event::{EventSender, Priority, GuiEvent, SharedGuiEvent};
use termio::gui::event;
use util::any::Upcast;
use std::any::Any;


pub trait Model: 'static + Send + Sync + Upcast<dyn Any> {
    fn make_gui(&mut self, username: &Name, node: NodeStrong) -> Gui;
}

pub trait ModelExt: Model {
    fn new_shared_event(f: impl 'static + Send + Sync + Fn(&mut Self)) -> SharedGuiEvent where Self: Sized {
        SharedGuiEvent::new(move |c: &mut NetcatController| f((&mut *c.model).upcast_mut().downcast_mut().unwrap()))
    }
    fn new_event(f: impl 'static + Send + Sync + FnOnce(&mut Self)) -> GuiEvent where Self: Sized {
        GuiEvent::new(move |c: &mut NetcatController| f((&mut *c.model).upcast_mut().downcast_mut().unwrap()))
    }
}

impl<T: Model> ModelExt for T {}

pub struct NetcatController {
    peers: HashMap<Name, Peer>,
    streams: HashSet<Shared<TcpStream>>,
    model: Box<dyn Model>,
}

impl Controller for NetcatController {}

struct Peer {
    stream: Shared<TcpStream>,
    gui: Gui,
    terminate: Arc<Condvar>,
}

pub struct NetcatServer {
    listener: TcpListener,
    event_sender: EventSender,
    controller: Arc<Mutex<NetcatController>>,
}

impl NetcatServer {
    pub fn new<T: Model>(address: &str, model: impl Fn(EventSender) -> T) -> io::Result<Arc<Self>> {
        let (event_sender, event_receiver) = event::channel();
        let controller = Arc::new(Mutex::new(NetcatController {
            peers: HashMap::new(),
            streams: HashSet::new(),
            model: Box::new(model(event_sender.clone())),
        }));
        let result = Arc::new(NetcatServer {
            listener: TcpListener::bind(address)?,
            event_sender,
            controller: controller.clone(),
        });
        event_receiver.start(controller);
        Ok(result)
    }
}

impl NetcatServer {
    fn handle_stream(self: &Arc<Self>,
                     mut stream: Shared<TcpStream>)
                     -> Result<(), Box<dyn error::Error>> {
        println!("Handling Stream");
        let (host, _) = proxy::run_proxy_server(&mut stream)?;
        let username = Arc::new(host.to_string()?);
        let username1 = username.clone();
        let username2 = username.clone();
        let (dirty_sender, dirty_receiver) = lossy::channel();
        let mut lock = self.controller.lock().unwrap();
        let node = NodeStrong::<dyn ViewImpl>::root(
            self.event_sender.clone(),
            move || dirty_sender.send(()),
            move |c: &NetcatController| &c.peers.get(&username1).unwrap().gui,
            move |c: &mut NetcatController| &mut c.peers.get_mut(&username2).unwrap().gui,
        );
        loop {
            let lock_mut = &mut *lock;
            let condvar = match lock_mut.peers.entry(username.clone()) {
                Entry::Occupied(peer) => {
                    peer.get().stream.shutdown(Shutdown::Read).ok();
                    peer.get().terminate.clone()
                }
                Entry::Vacant(x) => {
                    let gui = lock_mut.model.make_gui(&username, node);
                    x.insert(Peer {
                        stream: stream.clone(),
                        gui,
                        terminate: Arc::new(Condvar::new()),
                    });
                    break;
                }
            };
            lock = condvar.wait(lock).unwrap();
        }
        println!("Inserted");
        mem::drop(lock);
        let send_handle = thread::spawn({
            let username = username.clone();
            let mut stream = stream.clone();
            let controller = self.controller.clone();
            move || {
                let mut buffer = vec![];
                for () in dirty_receiver {
                    println!("Painting");
                    let enabled;
                    {
                        let mut lock = controller.lock().unwrap();
                        let peer = lock.peers.get_mut(&username).unwrap();
                        peer.gui.paint_buffer(&mut buffer);
                        enabled = peer.gui.enabled();
                    }
                    if let Err(e) = stream.write_all(&buffer) {
                        eprintln!("Send error {:?}", e);
                        break;
                    }
                    buffer.clear();
                    if !enabled {
                        break;
                    }
                }
                stream.shutdown(Shutdown::Both).ok();
            }
        });
        {
            let mut event_reader = EventReader::new(stream.clone());
            loop {
                match event_reader.read() {
                    Ok(event) => {
                        println!("Event {:?}", event);
                        let username3 = username.clone();
                        self.event_sender.run(
                            Priority::Later,
                            GuiEvent::new(move |c: &mut NetcatController| {
                                c.peers.get_mut(&username3).unwrap().gui.handle(&event)
                            }));
                    }
                    Err(error) => {
                        if error.kind() == ErrorKind::UnexpectedEof {
                            println!("Peer {:?} shutdown", username);
                        } else {
                            println!("Peer {:?} failed: {:?}", username, error);
                        }
                        break;
                    }
                }
            }
        }
        {
            let mut lock = self.controller.lock().unwrap();
            let peer = lock.peers.get_mut(&username).unwrap();
            peer.gui.set_enabled(false);
        }
        send_handle.join().unwrap();
        {
            let mut lock = self.controller.lock().unwrap();
            lock.peers.remove(&username).unwrap().terminate.notify_all();
        }
        Ok(())
    }

    pub fn listen(self: &Arc<Self>, context: Context) -> io::Result<()> {
        let self2 = self.clone();
        let on_cancel = context.on_cancel(move || {
            let mut lock = self2.controller.lock().unwrap();
            self2.listener.set_nonblocking(true).unwrap();
            for peer in lock.peers.values_mut() {
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
            let mut lock = self.controller.lock().unwrap();
            lock.streams.insert(stream.clone());
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
                self2.controller.lock().unwrap().streams.remove(&stream);
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