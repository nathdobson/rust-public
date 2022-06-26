use std::collections::HashMap;
use std::iter;
use std::ops::Deref;

use arrayvec::ArrayVec;
use id_collections::{id_type, IdMap, IdVec};
use itertools::Itertools;
use safe_cell::SafeLazy;
use serde::{Deserialize, Serialize};

use crate::WORD_WIDTH;

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub enum Color {
    BLACK = 0,
    GREEN = 1,
    YELLOW = 2,
}

#[derive(Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash, Debug, Serialize, Deserialize)]
pub struct HintValue([Color; WORD_WIDTH]);

#[id_type]
#[derive(Serialize, Deserialize)]
pub struct Hint(u8);

impl Color {
    pub fn distance(self) -> u8 {
        match self {
            Color::BLACK => 2,
            Color::GREEN => 0,
            Color::YELLOW => 1,
        }
    }
}

impl HintValue {
    pub fn new(x: [Color; WORD_WIDTH]) -> Self { HintValue(x) }
    pub fn distance(&self) -> u8 { self.0.iter().map(|x| x.distance()).sum() }
    pub fn hint(self) -> Hint { *HINT_TABLE.value_to_id.get(&self).unwrap() }
}

impl Deref for Hint {
    type Target = HintValue;

    fn deref(&self) -> &Self::Target { &HINT_TABLE.id_to_value[*self] }
}

impl Hint {
    pub fn index(self) -> usize { self.0 as usize }
}

struct HintTable {
    id_to_value: IdVec<Hint, HintValue>,
    value_to_id: HashMap<HintValue, Hint>,
}

static HINT_TABLE: SafeLazy<HintTable> = SafeLazy::new(make_hint_table);
fn make_hint_table() -> HintTable {
    let mut id_to_value = IdVec::new();
    let mut value_to_id = HashMap::new();
    for hv in iter::repeat([Color::BLACK, Color::GREEN, Color::YELLOW].into_iter())
        .take(WORD_WIDTH)
        .multi_cartesian_product()
    {
        let hv = HintValue::new(
            hv.into_iter()
                .collect::<ArrayVec<Color, WORD_WIDTH>>()
                .into_inner()
                .unwrap(),
        );
        let h = id_to_value.push(hv);
        value_to_id.insert(hv, h);
    }
    HintTable {
        id_to_value,
        value_to_id,
    }
}
