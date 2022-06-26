use id_collections::{IdMap, IdVec};
use safe_cell::SafeLazy;
use serde::{Deserialize, Serialize};

use crate::word::WordValue;
use crate::{memoize, Color, Hint, HintValue, Word, WORD_WIDTH};

#[derive(Serialize, Deserialize)]
pub struct PairHints {
    by_guess_solution: IdVec<Word, IdVec<Word, Hint>>,
}

pub static PAIR_HINTS: SafeLazy<PairHints> = SafeLazy::new(PairHints::generate_or_read);

impl PairHints {
    fn generate_hint(guess: WordValue, solution: WordValue) -> HintValue {
        let mut hint = [Color::BLACK; WORD_WIDTH];
        let mut solution_count = [0u8; 26];
        for c in solution.letters().iter() {
            solution_count[c.index()] += 1;
        }
        for i in 0..WORD_WIDTH {
            if guess.letters()[i] == solution.letters()[i] {
                solution_count[solution.letters()[i].index()] -= 1;
                hint[i] = Color::GREEN;
            }
        }
        for i in 0..WORD_WIDTH {
            if hint[i] == Color::BLACK {
                let count = &mut solution_count[guess.letters()[i].index()];
                if *count > 0 {
                    hint[i] = Color::YELLOW;
                    *count -= 1;
                }
            }
        }
        HintValue::new(hint)
    }
    pub fn generate_or_read() -> Self { memoize("pair_hints.dat", Self::generate) }

    pub fn generate() -> Self {
        let mut by_guess_solution: IdVec<Word, IdVec<Word, Hint>> =
            IdVec::with_capacity(Word::count());
        for guess in Word::words() {
            let mut by_solution = IdVec::with_capacity(Word::count());
            for solution in Word::words() {
                let _ = by_solution.push(Self::generate_hint(*guess, *solution).hint());
            }
            let _ = by_guess_solution.push(by_solution);
        }
        PairHints { by_guess_solution }
    }
    pub fn lookup(&self, guess: Word, solution: Word) -> Hint {
        self.by_guess_solution[guess][solution]
    }
}
