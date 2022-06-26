use std::cmp::Ordering;
use std::collections::HashMap;
use std::time::Instant;

use rand::rngs::StdRng;
use rand::seq::IteratorRandom;
use rand::{thread_rng, SeedableRng};

use crate::edit_graph::EDIT_GRAPH;
use crate::pair_hint::PAIR_HINTS;
use crate::{Word, WORD_COUNT};

#[derive(Debug, Copy, Clone, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Strategy([Word; WORD_COUNT]);

#[derive(Debug, Copy, Clone)]
pub struct ScoredStrategy {
    strategy: Strategy,
    score: u64,
}

impl Eq for ScoredStrategy {}

impl PartialEq<Self> for ScoredStrategy {
    fn eq(&self, other: &Self) -> bool { self.score.eq(&other.score) }
}

impl PartialOrd<Self> for ScoredStrategy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { self.score.partial_cmp(&other.score) }
}

impl Ord for ScoredStrategy {
    fn cmp(&self, other: &Self) -> Ordering { self.score.cmp(&other.score) }
}

impl Strategy {
    pub fn new(mut x: [Word; WORD_COUNT]) -> Self {
        x.sort();
        Strategy(x)
    }
    pub fn random() -> Self {
        Strategy::new([(); WORD_COUNT].map(|_| Word::words().choose(&mut thread_rng()).unwrap()))
    }
    pub fn score(&self) -> u64 {
        let mut by_outcome = HashMap::<_, usize>::with_capacity(4096);
        let pair_hints = &*PAIR_HINTS;
        for solution in Word::words() {
            let hints = self.0.map(|guess| pair_hints.lookup(guess, solution));
            let hints = hints
                .into_iter()
                .fold(0u64, |x, y| x * 243 + y.index() as u64);
            *by_outcome.entry(hints).or_default() += 1;
        }
        let result = by_outcome
            .values()
            .into_iter()
            .map(|x| ((*x - 1) as u64) * ((*x - 1) as u64))
            .sum::<u64>();
        result
    }
    pub fn score_detailed(&self) {
        let mut by_outcome = HashMap::<_, usize>::new();
        let pair_hints = &*PAIR_HINTS;
        for solution in Word::words() {
            let hints = self.0.map(|guess| pair_hints.lookup(guess, solution));
            *by_outcome.entry(hints).or_default() += 1;
        }
        for x in by_outcome.values() {
            if *x > 1 {
                print!("{:?} ", x);
            }
        }
        println!();
    }
    pub fn similar1<'a>(self, count: usize) -> impl 'a + Iterator<Item = Self> {
        let edit_graph = &*EDIT_GRAPH;
        (1..count).flat_map(move |diff| {
            (0..WORD_COUNT).map(move |pos| {
                let mut edited = self.clone();
                edited.0[pos] = edit_graph.lookup(edited.0[pos], diff);
                edited
            })
        })
    }
    pub fn similar2<'a>(self, count: usize) -> impl 'a + Iterator<Item = Self> {
        self.similar1(count).flat_map(move |x| x.similar1(count))
    }
    pub fn similar3<'a>(self, count: usize) -> impl 'a + Iterator<Item = Self> {
        self.similar1(count)
            .flat_map(move |x| x.similar1(count))
            .flat_map(move |x| x.similar1(count))
    }
    pub fn similar4<'a>(self, count: usize) -> impl 'a + Iterator<Item = Self> {
        self.similar1(count)
            .flat_map(move |x| x.similar1(count))
            .flat_map(move |x| x.similar1(count))
            .flat_map(move |x| x.similar1(count))
    }
    pub fn words(&self) -> &[Word; WORD_COUNT] { &self.0 }
}

impl ScoredStrategy {
    pub fn new(strategy: Strategy) -> Self {
        Self {
            strategy,
            score: strategy.score(),
        }
    }
    pub fn nearest_better_from(self, mut it: impl Iterator<Item = Strategy>) -> Option<Self> {
        it.find_map(|x| {
            let scored = ScoredStrategy::new(x);
            if scored.score < self.score {
                Some(scored)
            } else {
                None
            }
        })
    }
    pub fn iterate_better_from<F: Fn(Strategy) -> I, I: Iterator<Item = Strategy>>(
        self,
        f: F,
    ) -> Option<Self> {
        let mut improved = self.nearest_better_from(f(self.strategy))?;
        while let Some(improved2) = improved.nearest_better_from(f(improved.strategy)) {
            improved = improved2;
        }
        Some(improved)
    }
    pub fn score(&self) -> u64 { self.score }
    pub fn strategy(&self) -> Strategy { self.strategy }
}

#[cfg(test)]
mod test {
    extern crate test;

    use std::env::set_current_dir;

    use test::Bencher;

    use crate::Strategy;

    #[bench]
    fn bench_score(b: &mut Bencher) {
        set_current_dir("../../").unwrap();
        Strategy::random().score();
        b.iter(|| Strategy::random().score())
    }
}
