use std::backtrace::Backtrace;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{fmt, fs};

#[derive(Default)]
struct ProfileNode {
    children: BTreeMap<String, ProfileNode>,
    value: usize,
}

pub struct ProfileInner {
    root: ProfileNode,
    filename: PathBuf,
}

#[derive(Clone)]
pub struct Profile(Arc<Mutex<ProfileInner>>);

impl Profile {
    pub fn new(filename: PathBuf) -> Self {
        Profile(Arc::new(Mutex::new(ProfileInner {
            filename,
            root: ProfileNode::default(),
        })))
    }
    pub fn add(&mut self, value: usize) {
        let mut lock = self.0.lock().unwrap();
        let mut node = &mut lock.root;
        for line in Backtrace::capture().to_string().rsplit("\n") {
            let line = line.splitn(2, ":").nth(1).unwrap_or("???");
            node = node
                .children
                .entry(line.to_string())
                .or_insert(ProfileNode::default());
        }
        node.value += value;
    }
    pub fn flush(&self) {
        let lock = self.0.lock().unwrap();
        fs::write(&lock.filename, lock.to_string()).unwrap();
    }
}

impl ProfileNode {
    fn render<W: fmt::Write>(&self, w: &mut W, stack: &str) -> fmt::Result {
        writeln!(w, "{} {}", stack, self.value)?;
        for (frame, inner) in self.children.iter() {
            inner.render(w, &format!("{};{}", stack, frame))?;
        }
        Ok(())
    }
}

impl Display for ProfileInner {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { self.root.render(f, "") }
}
