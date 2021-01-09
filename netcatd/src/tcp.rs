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
use crate::{Handler, proxy, EventLoop};
use util::watch::{Watchable, Watch};
use util::{Name, lossy, cancel, expect};
use std::collections::{HashMap, HashSet};
use util::dirty::dirty_loop;
use termio::screen::Screen;
use std::thread::JoinHandle;
use crate::proxy::Host;
use std::time::Duration;
use chrono::format::Item::Error;
use util::cancel::{Context, Cancel};
use util::cancel::RecvError::Cancelling;
use std::collections::hash_map::Entry;

struct Peer {
    stream: Shared<TcpStream>,
    sender: Option<lossy::Sender<()>>,
    terminate: Arc<Condvar>,
}

struct State {
    streams: HashSet<Shared<TcpStream>>,
    peers: HashMap<Name, Peer>,
}

pub struct NetcatServer {
    listener: TcpListener,
    state: Mutex<State>,
}

impl NetcatServer {
    pub fn new(address: &str) -> io::Result<Arc<Self>> {
        let result = Arc::new(NetcatServer {
            listener: TcpListener::bind(address)?,
            state: Mutex::new(State {
                streams: HashSet::new(),
                peers: HashMap::new(),
            }),
        });
        Ok(result)
    }
}

impl EventLoop for NetcatServer {
    fn peer_render(&self, username: &Name) {
        if let Some(peer) = self.state.lock().unwrap().peers.get(username) {
            if let Some(sender) = &peer.sender {
                sender.send(());
            }
        }
    }
    fn peer_shutdown(&self, username: &Name) {
        if let Some(peer) = self.state.lock().unwrap().peers.get_mut(username) {
            peer.stream.shutdown(Shutdown::Read).ok();
        }
    }
}

impl NetcatServer {
    fn handle_stream(self: &Arc<Self>,
                     mut stream: Shared<TcpStream>,
                     handler: Arc<Mutex<dyn Handler>>)
                     -> Result<(), Box<dyn error::Error>> {
        let (host, _) = proxy::run_proxy_server(&mut stream)?;
        let username = Arc::new(host.to_string()?);
        let (sender, receiver) = lossy::channel();
        let mut lock = self.state.lock().unwrap();
        loop {
            let condvar = match lock.peers.entry(username.clone()) {
                Entry::Occupied(peer) => {
                    peer.get().stream.shutdown(Shutdown::Read).ok();
                    peer.get().terminate.clone()
                }
                Entry::Vacant(x) => {
                    x.insert(Peer {
                        stream: stream.clone(),
                        sender: Some(sender),
                        terminate: Arc::new(Condvar::new()),
                    });
                    break;
                }
            };
            lock = condvar.wait(lock).unwrap();
        }
        mem::drop(lock);
        let send_handle = thread::spawn({
            let handler = handler.clone();
            let username = username.clone();
            let mut stream = stream.clone();
            move || {
                let mut buffer = vec![];
                for () in receiver {
                    handler.lock().unwrap().peer_render(&username, &mut buffer);
                    if let Err(e) = stream.write_all(&buffer) {
                        eprintln!("Send error {:?}", e);
                        break;
                    }
                    buffer.clear();
                }
                stream.shutdown(Shutdown::Both).ok();
            }
        });
        {
            handler.lock().unwrap().peer_add(&username);
            let mut event_reader = EventReader::new(stream.clone());
            loop {
                match event_reader.read() {
                    Ok(event) => handler.lock().unwrap().peer_event(&username, &event),
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
        handler.lock().unwrap().peer_shutdown(&username);
        self.state.lock().unwrap().peers.get_mut(&username).unwrap().sender = None;
        send_handle.join().unwrap();
        handler.lock().unwrap().peer_close(&username);
        let mut lock = self.state.lock().unwrap();
        let peer = lock.peers.remove(&username).unwrap();
        peer.terminate.notify_all();
        Ok(())
    }

    pub fn listen(self: &Arc<Self>, context: Context, handler: Arc<Mutex<dyn Handler>>) -> io::Result<()> {
        let self2 = self.clone();
        let on_cancel = context.on_cancel(move || {
            let lock = self2.state.lock().unwrap();
            self2.listener.set_nonblocking(true).unwrap();
            for stream in lock.streams.iter() {
                stream.shutdown(Shutdown::Read).ok();
            }
            mem::drop(lock);
            TcpStream::connect(self2.listener.local_addr().unwrap()).ok();
        });
        for stream_result in self.listener.incoming() {
            if context.check().is_err() {
                return Ok(());
            }
            let stream = Shared::new(stream_result?);
            let mut lock = self.state.lock().unwrap();
            lock.streams.insert(stream.clone());
            let self2 = self.clone();
            let handler = handler.clone();
            context.spawn(move || -> cancel::Result<()>{
                if let Err(e) = self2.handle_stream(stream.clone(), handler) {
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
                self2.state.lock().unwrap().streams.remove(&stream);
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