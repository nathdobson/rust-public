use std::{error, io, mem, thread};
use std::any::Any;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fmt::{Debug, Formatter};
use std::future::Future;
use std::pin::Pin;
use std::str;
use std::sync::Arc;
use std::time::Duration;

use by_address::ByAddress;
use chrono::format::Item::Error;
use async_util::coop::{Cancel, Canceled};
use async_util::promise::Promise;
use termio::gui::event::{EventSender, GuiEvent, GuiPriority, read_loop, SharedGuiEvent};
use termio::gui::event;
use termio::gui::gui::Gui;
use termio::gui::tree::{Dirty, Tree};
use termio::input::{Event, EventReader};
use termio::screen::Screen;
use util::{expect, lossy, Name};
use util::dirty::dirty_loop;
use util::io::{pipeline, SafeWrite};
use util::mutrc::MutRc;
use util::shared::{Object, Shared};
use util::socket::{set_linger, set_reuse_port};
use tokio::join;

use crate::proxy;
use crate::proxy::Host;
use tokio::net::TcpListener;
use tokio::net::TcpStream;
use std::io::ErrorKind;
use async_util::spawn::Spawn;
use async_backtrace::trace_annotate;

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
    peer_cancel: Cancel,
    terminated: Promise<!>,
}

pub struct NetcatServerBuilder {
    pub server_cancel: Cancel,
    pub event_sender: EventSender,
}

pub struct NetcatServer {
    listener: TcpListener,
    event_sender: EventSender,
    state: MutRc<NetcatState>,
    model: MutRc<dyn Model>,
    server_cancel: Cancel,
}

impl NetcatServerBuilder {
    pub fn new() -> (Self, impl Future<Output=()>) {
        let server_cancel = Cancel::new();
        let (event_sender, event_joiner) =
            event::event_loop();
        (NetcatServerBuilder {
            server_cancel,
            event_sender,
        }, event_joiner)
    }

    async fn build_inner(self, address: &str, model: MutRc<dyn Model>)
                         -> io::Result<Arc<NetcatServer>> {
        NetcatServer::new(
            address,
            self.event_sender,
            model,
            self.server_cancel,
        ).await
    }

    async fn run(self, address: &str, model: MutRc<dyn Model>) -> io::Result<()> {
        let server = self.build_inner(address, model).await?;
        server.listen().await?;
        Ok(())
    }

    async fn run_main(self, address: &str, model: MutRc<dyn Model>) -> ! {
        self.server_cancel.clone().run_main(Duration::from_secs(5), self.run(address, model)).await
    }

    pub fn build_main(self, address: &str, model: MutRc<dyn Model>) {
        let address = address.to_string();
        self.event_sender.clone().spawner().with_priority(GuiPriority::Read).spawn(async move {
            trace_annotate(&|f| write!(f, "Listening {}", address), self.run_main(&address, model)).await;
        })
    }
}

impl NetcatServer {
    pub async fn new(
        address: &str,
        event_sender: EventSender,
        model: MutRc<dyn Model>,
        server_cancel: Cancel) -> io::Result<Arc<Self>> {
        let state = MutRc::new(NetcatState {
            peers: HashMap::new(),
        });
        let result = Arc::new(NetcatServer {
            listener: TcpListener::bind(address).await?,
            event_sender,
            state,
            model,
            server_cancel,
        });
        Ok(result)
    }
    async fn handle_stream(self: &Arc<Self>,
                           mut stream: TcpStream)
                           -> Result<(), Box<dyn error::Error>> {
        let peer_cancel = Cancel::new();
        let (host, _) = proxy::run_proxy_server(&mut stream).await?;
        let (read_stream, mut write_stream) = stream.into_split();
        let username = Arc::new(host.to_string()?);
        let (tree, paint, layout) =
            Tree::new(peer_cancel.clone(), self.event_sender.clone());
        peer_cancel.attach(&self.server_cancel);
        loop {
            let terminated: Promise<!> = match self.state.borrow_mut().peers.entry(username.clone()) {
                Entry::Occupied(peer) => {
                    eprintln!("Peer {:?} reconnecting", username);
                    peer.get().peer_cancel.cancel();
                    peer.get().terminated.clone()
                }
                Entry::Vacant(x) => {
                    eprintln!("Peer {:?} connected", username);
                    x.insert(Peer {
                        peer_cancel: peer_cancel.clone(),
                        terminated: Promise::new(),
                    });
                    break;
                }
            };
            terminated.join().await;
            eprintln!("Peer {:?} reconnected", username);
        }
        let gui = self.model.borrow_mut().add_peer(&username, tree.clone());

        let layout_annot = {
            let username = username.clone();
            move |f: &mut Formatter| write!(f, "User {} layout loop", username)
        };
        let write_annot = {
            let username = username.clone();
            move |f: &mut Formatter| write!(f, "User {} send loop", username)
        };
        let read_annot = {
            let username = username.clone();
            move |f: &mut Formatter| write!(f, "User {} receive loop", username)
        };

        let ll = {
            let gui = gui.clone();
            async move {
                trace_annotate(&layout_annot, layout.layout_loop(gui)).await
            }
        };
        let wl = {
            let gui = gui.clone();
            let peer_cancel = peer_cancel.clone();
            let username = username.clone();
            async move {
                trace_annotate(&write_annot, async move {
                    if let Err(e) = paint.render_loop(gui, Pin::new(&mut write_stream)).await {
                        eprintln!("Peer {:?} responses error: {}", username, e);
                    }
                    peer_cancel.cancel();
                }).await
            }
        };
        let rl = {
            let peer_cancel = peer_cancel.clone();
            let event_sender = self.event_sender.clone();
            let username = username.clone();

            async move {
                trace_annotate(&read_annot, async move {
                    match peer_cancel.checked(read_loop(
                        event_sender,
                        gui.clone(),
                        read_stream)).await {
                        Ok(Ok(x)) => match x {}
                        Ok(Err(error)) => {
                            if error.kind() == ErrorKind::UnexpectedEof {
                                eprintln!("Peer {:?} requests EOF", username);
                            } else {
                                eprintln!("Peer {:?} requests error: {}", username, error);
                            }
                        }
                        Err(_canceled) => {
                            eprintln!("Peer {:?} requests canceled", username);
                        }
                    }
                    gui.borrow_mut().set_enabled(false);
                    gui.borrow_mut().tree().cancel().cancel();
                }).await;
            }
        };
        trace_annotate(
            &{
                let username = username.clone();
                move |f| write!(f, "User {}", username)
            },
            async move {
                join!(
                    self.event_sender.spawner().with_priority(GuiPriority::Read).spawn_with_handle(rl),
                    self.event_sender.spawner().with_priority(GuiPriority::Layout).spawn_with_handle(ll),
                    self.event_sender.spawner().with_priority(GuiPriority::Paint).spawn_with_handle(wl)
                )
            },
        ).await;
        eprintln!("Peer {:?} removed", username);
        self.state.borrow_mut().peers.remove(&username).unwrap();
        self.model.borrow_mut().remove_peer(&username);
        Ok(())
    }

    pub async fn listen(self: &Arc<Self>) -> io::Result<()> {
        let bundle = Promise::new();
        loop {
            let (stream, addr) =
                match self.server_cancel.checked(self.listener.accept()).await {
                    Err(Canceled) => break,
                    Ok(stream_result) => stream_result?,
                };
            let self2 = self.clone();
            self.event_sender.spawner().with_priority(GuiPriority::Read).spawn(bundle.outlive(async move {
                trace_annotate(&|f| write!(f, "Peer {}", addr), async move {
                    if let Err(e) = self2.handle_stream(stream).await {
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
                }).await
            }));
        }
        eprintln!("Server canceling all peers");
        bundle.join().await;
        eprintln!("Server canceled all peers");
        Ok(())
    }
}

