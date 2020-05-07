use std::backtrace::Backtrace;
use std::collections::BTreeMap;
use std::fmt::Display;
use std::fmt;

pub struct Profile {
    children: BTreeMap<String, Profile>,
    value: usize,
}

impl Profile {
    pub fn new() -> Self {
        Profile {
            children: BTreeMap::new(),
            value: 0,
        }
    }
    pub fn add(&mut self, value: usize) {
        let mut node = self;
        for line in Backtrace::capture().to_string().split("\n") {
            node = node.children.entry(line.to_string()).or_insert(Profile::new());
            node.value += value;
        }
    }
    fn render<W: fmt::Write>(&self, w: &mut W, stack: &str) -> fmt::Result {
        writeln!(w, "{} {}", stack, self.value)?;
        for (frame, inner) in self.children.iter() {
            inner.render(w, &format!("{};{}", stack, frame))?;
        }
        Ok(())
    }
}

impl Display for Profile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.render(f, "")
    }
}