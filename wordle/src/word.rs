use std::collections::HashMap;
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Deref;

use arrayvec::ArrayVec;
use id_collections::{id_type, IdMap, IdVec};
use safe_cell::SafeLazy;
use serde::{Deserialize, Serialize};

use crate::{Color, HintValue, WORD_COUNT, WORD_WIDTH};

#[id_type]
#[derive(Serialize, Deserialize)]
pub struct Letter(u8);

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct WordValue([Letter; WORD_WIDTH]);

#[id_type(debug = false)]
#[derive(Serialize, Deserialize)]
pub struct Word(u16);

impl Letter {
    pub fn new(x: u8) -> Self {
        assert!(b'a' <= x && x <= b'z');
        Letter(x - b'a')
    }
    pub fn index(self) -> usize { self.0 as usize }
}

impl WordValue {
    pub fn new(bytes: &[u8]) -> Self {
        WordValue(
            bytes
                .into_iter()
                .map(|x| Letter::new(*x))
                .collect::<ArrayVec<Letter, WORD_WIDTH>>()
                .into_inner()
                .unwrap(),
        )
    }
    pub fn letters(&self) -> &[Letter; WORD_WIDTH] { &self.0 }
}

impl Word {
    pub fn nth(x: usize) -> Self { Word(x as u16) }
    pub fn count() -> usize { WORD_TABLE.id_to_value.len() }
    pub fn words() -> impl Iterator<Item = Word> {
        (0u16..WORD_TABLE.id_to_value.len() as u16).map(Word)
    }
}

impl Deref for Word {
    type Target = WordValue;

    fn deref(&self) -> &Self::Target { &WORD_TABLE.id_to_value[self] }
}

impl<'a> From<&'a [u8]> for Word {
    fn from(w: &'a [u8]) -> Self { *WORD_TABLE.value_to_id.get(&WordValue::new(w)).unwrap() }
}

impl Debug for WordValue {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        for x in self.0 {
            write!(f, "{}", (x.0 + b'a') as char)?;
        }
        Ok(())
    }
}

struct WordTable {
    id_to_value: IdVec<Word, WordValue>,
    value_to_id: HashMap<WordValue, Word>,
}

static WORD_TABLE: SafeLazy<WordTable> = SafeLazy::new(WordTable::new);

impl WordTable {
    fn new() -> Self {
        let mut id_to_value = IdVec::new();
        let mut value_to_id = HashMap::new();
        for line in BufReader::new(
            File::open(&format!("rust-public/wordle/words{}.txt", WORD_WIDTH)).unwrap(),
        )
        .lines()
        {
            let wv = WordValue::new(line.unwrap().as_bytes());
            let w = id_to_value.push(wv);
            value_to_id.insert(wv, w);
        }
        WordTable {
            id_to_value,
            value_to_id,
        }
    }
}

impl Debug for Word {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { (**self).fmt(f) }
}
