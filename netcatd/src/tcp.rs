use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::{HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::task::Context;
use std::time::Duration;
use std::{error, io, mem, str, thread};

use async_util::coop::{Cancel, Canceled};
use async_util::poll::PollResult::{Abort, Noop, Yield};
use async_util::poll::{poll_next, PollResult};
use async_util::promise::Promise;
use async_util::spawn::Spawn;
use by_address::ByAddress;
use termio::gui::gui::Gui;
use termio::gui::tree::{Dirty, Tree};
//use termio::gui::event::{EventSender, GuiEvent, GuiPriority, read_loop, SharedGuiEvent};
use termio::gui::{event, GuiBuilder};
use termio::input::{Event, EventReader};
use termio::screen::Screen;
use tokio::join;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio_stream::wrappers::UnboundedReceiverStream;
use util::dirty::dirty_loop;
use util::expect::Server;
use util::io::{pipeline, SafeWrite};
use util::mutrc::MutRc;
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};
use util::{expect, lossy, Name};

use crate::proxy;
use crate::proxy::Host;

pub trait Model {
    fn make_peer(&mut self, name: &Name, builder: GuiBuilder) -> Gui;
    fn remove_peer(&mut self, name: &Name);
}

#[derive(Debug)]
struct PeerRequest {
    name: Name,
    stream: TcpStream,
}

#[derive(Debug)]
struct Peer {
    gui: Gui,
}

pub struct NetcatServer {
    peers: HashMap<Name, Peer>,
    queue: HashMap<Name, TcpStream>,
    request_stream: Option<UnboundedReceiverStream<PeerRequest>>,
    server_cancel: Cancel,
}

pub struct NetcatListener {
    request_sender: UnboundedSender<PeerRequest>,
    server_cancel: Cancel,
}

impl NetcatListener {
    async fn accept(cancel: Cancel, mut stream: TcpStream, sender: UnboundedSender<PeerRequest>) {
        if let Err(e) = async {
            if let Ok((host, _port)) = cancel.checked(proxy::run_proxy_server(&mut stream)).await? {
                sender
                    .send(PeerRequest {
                        name: Arc::new(host.to_string()?),
                        stream,
                    })
                    .ok();
            }
            Ok::<_, Box<dyn std::error::Error>>(())
        }
        .await
        {
            eprintln!("proxy error: {}", e);
        }
    }
    pub async fn listen(self, address: &str) -> io::Result<impl Future> {
        let listener = TcpListener::bind(address).await?;
        let sender = self.request_sender.clone();
        let cancel = self.server_cancel.clone();
        Ok(async move {
            loop {
                match cancel.checked(listener.accept()).await {
                    Err(Canceled) => break,
                    Ok(Err(e)) => {
                        eprintln!("accept error: {}", e);
                        break;
                    }
                    Ok(Ok((stream, _))) => spawn(NetcatListener::accept(
                        cancel.clone(),
                        stream,
                        sender.clone(),
                    )),
                };
            }
            println!("Server stopped accepting");
        })
    }
}

impl NetcatServer {
    pub fn new(cancel: Cancel) -> (NetcatListener, Self) {
        let (tx, rx) = unbounded_channel();
        (
            NetcatListener {
                request_sender: tx,
                server_cancel: cancel.clone(),
            },
            NetcatServer {
                peers: HashMap::new(),
                queue: HashMap::new(),
                server_cancel: cancel,
                request_stream: Some(UnboundedReceiverStream::new(rx)),
            },
        )
    }

    fn make_peer(
        server_cancel: &Cancel,
        name: &Name,
        stream: TcpStream,
        model: &mut dyn Model,
    ) -> Peer {
        let mut builder = GuiBuilder::new();
        builder.tree().cancel().attach(server_cancel);
        let (read, write) = stream.into_split();
        builder.set_input(Box::pin(read));
        builder.set_output(Box::pin(write));
        let gui = model.make_peer(&name, builder);
        Peer { gui }
    }
    pub fn poll_elapse(
        &mut self,
        cx: &mut Context,
        model: &mut dyn Model,
    ) -> PollResult<(), io::Error> {
        poll_next(cx, &mut self.request_stream).map(|request| {
            match self.peers.entry(request.name.clone()) {
                Entry::Occupied(mut e) => {
                    println!("Peer {} reconnecting", request.name);
                    e.get_mut().gui.tree().cancel().cancel();
                    self.queue.insert(request.name, request.stream);
                }
                Entry::Vacant(e) => {
                    println!("Peer {} connected", request.name);
                    e.insert(Self::make_peer(
                        &self.server_cancel,
                        &request.name,
                        request.stream,
                        model,
                    ));
                }
            }
        })?;
        let names: Vec<Name> = self.peers.keys().cloned().collect();
        for name in names.iter() {
            if let Some(peer) = self.peers.get_mut(name) {
                match peer.gui.poll_elapse(cx) {
                    Noop => {}
                    Yield(()) => return Yield(()),
                    Abort(e) => {
                        if e.kind() != ErrorKind::Interrupted {
                            eprintln!("Peer {} error {}", name, e);
                        }
                        self.peers.remove(name);
                        model.remove_peer(name);
                        println!("Peer {} removed", name);
                        if let Some(new_stream) = self.queue.remove(name) {
                            println!("Peer {} reconnected", name);
                            self.peers.insert(
                                name.clone(),
                                Self::make_peer(&self.server_cancel, name, new_stream, model),
                            );
                        }
                    }
                }
            }
        }
        if self.request_stream.is_none() && self.peers.is_empty() && self.queue.is_empty() {
            println!("Server terminated");
            return Abort(io::Error::new(io::ErrorKind::Interrupted, "canceled"));
        }
        Noop
    }
}
