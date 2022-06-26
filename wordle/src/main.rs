#![allow(unused_imports)]
#![allow(dead_code)]
#![feature(try_blocks)]
#![feature(box_syntax)]
#![allow(unused_variables)]
#![deny(unused_must_use)]
#![feature(test)]
#![feature(let_chains)]

mod edit_graph;
mod hint;
mod pair_hint;
mod strategy;
mod word;

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::hash::Hash;
use std::io::{BufRead, BufReader};
use std::iter::Rev;
use std::mem::size_of;
use std::sync::Arc;
use std::time::Instant;
use std::{fs, iter, mem, thread};

use arrayvec::ArrayVec;
use id_collections::{id_type, Count, IdMap, IdVec};
use itertools::Itertools;
use parking_lot::Mutex;
use rand::distributions::{Distribution, Standard};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use serde::{Deserialize, Serialize};

use crate::hint::{Color, Hint, HintValue};
use crate::strategy::{ScoredStrategy, Strategy};
use crate::word::Word;

const WORD_WIDTH: usize = 5;
const WORD_COUNT: usize = 5;

#[derive(Debug, Serialize, Deserialize)]
struct HintMap {
    by_guess_solution: IdVec<Word, IdVec<Word, Hint>>,
    hints: IdMap<Hint, HintValue>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Pairs {
    pairs: Vec<(Word, Word, usize)>,
}

//
//
// impl Hint {
//     fn new(value: HintValue) -> Self {
//         let mut total = 0u8;
//         for c in value.0 {
//             total *= 3u8;
//             total += c as u8;
//         }
//         Hint::new(total)
//     }
// }

fn memoize<T: Serialize + for<'de> Deserialize<'de>>(
    filename: &str,
    callback: impl FnOnce() -> T,
) -> T {
    let filename = format!("rust-public/wordle/memoize{}_{}", WORD_WIDTH, filename);
    if let Ok(contents) = fs::read(&filename) {
        return bincode::deserialize(&contents).unwrap();
    }
    let value = callback();
    fs::write(&filename, bincode::serialize(&value).unwrap()).unwrap();
    value
}

// fn find_pairs(dict: &Dictionary, hint_map: &HintMap) -> Pairs {
//     let mut pairs = vec![];
//     let mut group_by_guess = IdVec::<Word, IdMap<Hint, Vec<Word>>>::new();
//     for (guess, _) in dict.words.iter() {
//         let mut group_by_hint = IdMap::<Hint, Vec<Word>>::new();
//         for (solution, _) in dict.words.iter() {
//             group_by_hint
//                 .entry(hint_map.by_guess_solution[guess][solution])
//                 .or_default()
//                 .push(solution)
//         }
//         let _ = group_by_guess.push(group_by_hint);
//     }
//     let group_by_guess: IdMap<Word, Vec<(Hint, Vec<Word>)>> = group_by_guess
//         .into_iter()
//         .map(|(guess, group_by_hint)| {
//             let mut group_by_hint: Vec<(Hint, Vec<Word>)> = group_by_hint.into_iter().collect();
//             group_by_hint.sort_by_key(|(_, words)| Reverse(Some(words.len())));
//             (guess, group_by_hint)
//         })
//         .collect();
//     let mut count_by_hint = IdVec::new();
//     let _ = count_by_hint.resize(Count::from_value(243), 0u8);
//     for (guess1, guess_value1) in dict.words.iter() {
//         println!("{:?}", guess1);
//         for (guess2, guess_value2) in dict.words.iter() {
//             if guess1 < guess2 {
//                 let _: Option<()> = try {
//                     let mut max = 0;
//                     for (hint, solutions) in group_by_guess.get(&guess1).unwrap() {
//                         count_by_hint.as_mut_slice().fill(0);
//                         for solution in solutions {
//                             let c =
//                                 &mut count_by_hint[hint_map.by_guess_solution[guess2][solution]];
//                             *c = c.checked_add(1)?;
//                         }
//                         max = max.max(count_by_hint.values().cloned().max().unwrap());
//                     }
//                     pairs.push((guess1, guess2, max as usize))
//                 };
//             }
//         }
//     }
//     pairs.sort_by_key(|x| x.2);
//     Pairs { pairs }
// }
//
// fn find_triples(pairs: &Pairs) -> Vec<(Word, Word, Word)> {
//     let mut adjacency = HashMap::<Word, HashSet<Word>>::new();
//     let mut triples = vec![];
//     for (guess1, guess2, _) in pairs.pairs.iter() {
//         adjacency.entry(*guess1).or_default().insert(*guess2);
//         adjacency.entry(*guess2).or_default().insert(*guess1);
//         let adj1 = adjacency.get(&guess1).unwrap();
//         let adj2 = adjacency.get(&guess2).unwrap();
//         for guess3 in adj1 {
//             if adj2.contains(guess3) {
//                 triples.push((*guess1, *guess2, *guess3));
//             }
//         }
//     }
//     triples
// }
//
// struct OutcomeMap<const N: usize> {
//     by_word: HashMap<[Word; N], usize>,
// }
//
// impl<const N: usize> OutcomeMap<N> {
//     pub fn new() -> Self {
//         OutcomeMap {
//             by_word: HashMap::new(),
//         }
//     }
//     pub fn insert(&mut self, dict: &Dictionary, hint_map: &HintMap, guesses: &[Word; N]) {
//         let mut by_outcome = HashMap::new();
//         for (solution, _) in dict.words.iter() {
//             let hints = guesses.map(|guess| hint_map.by_guess_solution[guess][solution]);
//             *by_outcome.entry(hints).or_default() += 1;
//         }
//         self.by_word
//             .insert(*guesses, by_outcome.values().cloned().max().unwrap());
//     }
//     pub fn ordered(&self) -> Vec<([Word; N], usize)> {
//         let mut vec: Vec<_> = self
//             .by_word
//             .iter()
//             .map(|(k, v)| (k.clone(), v.clone()))
//             .collect();
//         vec.sort_by_key(|x| x.1);
//         vec
//     }
// }
//
// fn find_triples_by_max(
//     dict: &Dictionary,
//     hint_map: &HintMap,
//     triples: &[(Word, Word, Word)],
// ) -> Vec<(Word, Word, Word, usize)> {
//     let mut triples_by_max = vec![];
//     for (index, (guess1, guess2, guess3)) in triples.iter().enumerate() {
//         if index % 1000 == 0 {
//             println!("{:?}/{:?}", index, triples.len());
//         }
//         let mut outcomes = HashMap::<_, usize>::new();
//         for (solution, _) in dict.words.iter() {
//             *outcomes
//                 .entry((
//                     hint_map.by_guess_solution[guess1][solution],
//                     hint_map.by_guess_solution[guess2][solution],
//                     hint_map.by_guess_solution[guess3][solution],
//                 ))
//                 .or_default() += 1;
//         }
//         let max = outcomes.values().cloned().max().unwrap();
//         triples_by_max.push((*guess1, *guess2, *guess3, max));
//     }
//     triples_by_max.sort_by_key(|x| x.3);
//     triples_by_max
// }
//
// #[derive(Debug, Ord, PartialEq, PartialOrd, Eq, Hash, Serialize, Deserialize, Clone, Copy)]
// struct SortedPair<T>(T, T);
//
// impl<T: Ord> SortedPair<T> {
//     pub fn new(x: T, y: T) -> Self {
//         if x <= y {
//             SortedPair(x, y)
//         } else {
//             SortedPair(y, x)
//         }
//     }
// }
//

struct QueueInner {
    queue: BinaryHeap<Reverse<ScoredStrategy>>,
}

#[derive(Clone)]
struct Queue(Arc<Mutex<QueueInner>>);

impl Queue {
    pub fn new() -> Self {
        Queue(Arc::new(Mutex::new(QueueInner {
            queue: BinaryHeap::new(),
        })))
    }
    pub fn send(&self, strat: ScoredStrategy) {
        let ref mut lock = self.0.lock();
        lock.queue.push(Reverse(strat));
    }
    pub fn recv(&self) -> Option<ScoredStrategy> { self.0.lock().queue.pop().map(|x| x.0) }
    pub fn recv_or_random(&self) -> ScoredStrategy {
        self.recv()
            .unwrap_or_else(|| ScoredStrategy::new(Strategy::random()))
    }
}

struct QueueSet {
    queues: Vec<Queue>,
    used: Mutex<HashSet<Strategy>>,
}

impl QueueSet {
    pub fn send(&self, strat: ScoredStrategy, exclude: Option<usize>) {
        if self.used.lock().insert(strat.strategy()) {
            for (index, queue) in self.queues.iter().enumerate() {
                if Some(index) != exclude {
                    queue.send(strat);
                }
            }
        } else {
            //println!("repeat");
        }
    }
}

fn execute(fs: Vec<Box<dyn Send + FnMut(ScoredStrategy) -> Option<ScoredStrategy>>>) {
    let mut queues = vec![];
    for i in 0..fs.len() {
        queues.push(Queue::new());
    }
    let queues = Arc::new(QueueSet {
        queues,
        used: Mutex::new(HashSet::new()),
    });
    let mut handles = vec![];
    let best = Arc::new(Mutex::new(u64::MAX));
    handles.push({
        let queues = queues.clone();
        thread::spawn(move || loop {
            let new = ScoredStrategy::new(Strategy::random());
            if new.score() < 30000 {
                queues.send(new, None);
            }
        })
    });
    for (index, mut f) in fs.into_iter().enumerate() {
        let queues = queues.clone();
        let best = best.clone();
        handles.push(thread::spawn(move || loop {
            let next = queues.queues[index].recv_or_random();
            if let Some(improved) = f(next) {
                if improved.score() < 4000 {
                    let ref mut best = *best.lock();
                    if improved.score() < *best {
                        *best = improved.score();
                        mem::drop(best);
                        println!("{:?} {:?}", index, improved);
                    }
                }
                queues.send(improved, Some(index));
            }
        }));
    }
    for handle in handles {
        handle.join().unwrap();
    }
}

fn main() {
    execute(vec![
        box |strat| strat.iterate_better_from(|strat| strat.similar1(10)),
        box |strat| strat.iterate_better_from(|strat| strat.similar1(100)),
        box |strat| strat.iterate_better_from(|strat| strat.similar1(1000)),
        box |strat| strat.iterate_better_from(|strat| strat.similar1(10000)),
        box |strat| strat.iterate_better_from(|strat| strat.similar2(10)),
        box |strat| strat.iterate_better_from(|strat| strat.similar2(100)),
        box |strat| strat.iterate_better_from(|strat| strat.similar3(5)),
        box |strat| strat.iterate_better_from(|strat| strat.similar3(10)),
        box |strat| strat.iterate_better_from(|strat| strat.similar4(5)),
        box |strat| strat.iterate_better_from(|strat| strat.similar4(10)),
        {
            let mut prev: BinaryHeap<ScoredStrategy> = BinaryHeap::new();
            box move |next: ScoredStrategy| {
                let mut best: Option<ScoredStrategy> = None;
                for prev in prev.iter().cloned() {
                    for prev in prev.strategy().words().iter().cloned().powerset() {
                        for next in next.strategy().words().iter().cloned().powerset() {
                            if prev.len() > 0
                                && next.len() > 0
                                && next.len() + prev.len() == WORD_COUNT
                            {
                                let strategy = Strategy::new(
                                    prev.iter()
                                        .cloned()
                                        .chain(next.into_iter())
                                        .collect::<ArrayVec<_, WORD_COUNT>>()
                                        .into_inner()
                                        .unwrap(),
                                );
                                let strategy = ScoredStrategy::new(strategy);
                                if let Some(best) = &mut best {
                                    *best = (*best).min(strategy);
                                } else {
                                    best = Some(strategy);
                                }
                            }
                        }
                    }
                }
                prev.push(next);
                if prev.len() > 50 {
                    prev.pop();
                }
                println!(" {:?} {:?}", next, best);
                best
            }
        },
    ]);
    //let dict = memoize("dict.data", Dictionary::read);
    // let hint_map = memoize("hints.data", || HintMap::new(&dict));
    // let edit_graph = memoize("edit_graph.dat", || EditGraph::new(&dict, &hint_map));

    // let mut best_score = u64::MAX;
    // for seed in 1000..2000 {
    //     println!("{:?}", seed);
    //     let mut strat = ScoredStrategy::new(Strategy::random());
    //     while let Some(better) = strat.nearest_better() {
    //         strat = better;
    //     }
    //     if strat.score() < 1300 {
    //         while let Some(better) = strat.nearest_better_from(
    //             strat
    //                 .strategy()
    //                 .similar3()
    //                 .chain(strat.strategy().similar4()),
    //         ) {
    //             println!("extra improvement: {:?}", strat);
    //             strat = better;
    //         }
    //     }
    //     if strat.score() < best_score {
    //         best_score = strat.score();
    //         println!("{:?}", strat);
    //         strat.strategy().score_detailed();
    //     }
    // }
    // let pairs = memoize("pairs.data", || find_pairs(&dict, &hint_map));
    // let triples = memoize("triples.data", || find_triples(&pairs));
    // let triples_by_max = memoize("triples_by_max.data", || {
    //     find_triples_by_max(&dict, &hint_map, &triples)
    // });
    // let mut adjacency = HashMap::<SortedPair<Word>, HashMap<Word, usize>>::new();
    // let mut outcomes = OutcomeMap::new();
    // for (index, (g1, g2, g3, max)) in triples_by_max.iter().enumerate() {
    //     if index % 1000 == 0 {
    //         println!("{:?}/{:?}", index, triples_by_max.len());
    //     }
    //     let g12 = SortedPair::new(*g1, *g2);
    //     let g13 = SortedPair::new(*g1, *g3);
    //     let g23 = SortedPair::new(*g2, *g3);
    //     adjacency.entry(g12).or_default().insert(*g3, *max);
    //     adjacency.entry(g13).or_default().insert(*g2, *max);
    //     adjacency.entry(g23).or_default().insert(*g1, *max);
    //     let s12 = adjacency.get(&g12).unwrap();
    //     let s23 = adjacency.get(&g23).unwrap();
    //     let s13 = adjacency.get(&g13).unwrap();
    //     for (g4, m12) in s12 {
    //         if let (Some(m23), Some(m13)) = (s23.get(g4), s13.get(g4)) {
    //             // println!("{:?} {:?} {:?} {:?}", m12, m23, m13, max);
    //             outcomes.insert(&dict, &hint_map, &[*g1, *g2, *g3, *g4]);
    //         }
    //     }
    // }
    // for (words, max) in outcomes.ordered().iter().take(10) {
    //     println!("{:?} {:?}", words.map(|x| dict.words[x]), max);
    // }
    // let g1 = dict.get(b"lupin");
    // let g2 = dict.get(b"roted");
    // let g3 = dict.get(b"samek");
    // let g4 = dict.get(b"chevy");
    // let mut outcomes = OutcomeMap::new();
    // for (g5, _) in dict.words.iter() {
    //     outcomes.insert(&dict, &hint_map, &[g1, g2, g3, g4, g5])
    // }
    // println!("{:?}", outcomes.ordered().first());
    // for (g1, g2, g3, c) in triples_by_max.iter().take(100) {
    //     println!(
    //         "{:?} {:?} {:?} {:?}",
    //         dict.words[g1], dict.words[g2], dict.words[g3], c
    //     );
    //     let mut outcomes = OutcomeMap::new();
    //     for (g4, _) in &dict.words {
    //         outcomes.insert(&dict, &hint_map, &[*g1, *g2, *g3, g4]);
    //     }
    //     for (words, max) in outcomes.ordered().iter().take(1) {
    //         println!("{:?} {:?}", words.map(|x| dict.words[x]), max);
    //     }
    // }
}
