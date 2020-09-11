use std::collections::VecDeque;
use std::ops::Index;

#[derive(Clone, Copy, Eq, PartialOrd, PartialEq, Ord, Debug)]
pub struct QueueKey(u64);

pub struct Queue<T> {
    front: QueueKey,
    queue: VecDeque<T>,
}

impl<T> Queue<T> {
    pub fn new() -> Self {
        Queue {
            front: QueueKey(0),
            queue: VecDeque::new(),
        }
    }
    fn index(&self, key: QueueKey) -> Option<usize> {
        if self.front.0 <= key.0 && key.0 - self.front.0 <= self.queue.len() as u64 {
            Some((key.0 - self.front.0) as usize)
        } else {
            None
        }
    }

    pub fn get(&self, key: QueueKey) -> Option<&T> {
        if let Some(index) = self.index(key) {
            Some(&self.queue[index])
        } else {
            None
        }
    }

    pub fn get_mut(&mut self, key: QueueKey) -> Option<&mut T> {
        if let Some(index) = self.index(key) {
            Some(&mut self.queue[index])
        } else {
            None
        }
    }

    pub fn push_back(&mut self, value: T) -> QueueKey {
        let key = self.front.0 + self.queue.len() as u64;
        self.queue.push_back(value);
        QueueKey(key)
    }
    pub fn front(&self) -> Option<&T> {
        self.queue.front()
    }
    pub fn front_key(&self) -> QueueKey {
        self.front
    }
    pub fn pop_front(&mut self) -> Option<T> {
        if let Some(value) = self.queue.pop_front() {
            self.front.0 += 1;
            Some(value)
        }else{
            None
        }
    }
}