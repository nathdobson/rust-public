use id_collections::IdVec;
use safe_cell::SafeLazy;
use serde::{Deserialize, Serialize};

use crate::pair_hint::PAIR_HINTS;
use crate::{memoize, Word};

#[derive(Serialize, Deserialize)]
pub struct EditGraph {
    edits: IdVec<Word, Vec<(Word, u8)>>,
}

pub static EDIT_GRAPH: SafeLazy<EditGraph> = SafeLazy::new(EditGraph::generate_or_read);

impl EditGraph {
    fn generate() -> EditGraph {
        let mut edits = IdVec::new();
        let pair_hints = &*PAIR_HINTS;
        for word1 in Word::words() {
            let mut edits_by_word = Vec::new();
            for word2 in Word::words() {
                let hint = pair_hints.lookup(word1, word2);
                edits_by_word.push((word2, hint.distance()));
            }
            edits_by_word.sort_by_key(|x| x.1);
            let _ = edits.push(edits_by_word);
        }
        EditGraph { edits }
    }
    fn generate_or_read() -> Self { memoize("edit_graph.dat", Self::generate) }
    pub fn lookup(&self, word: Word, index: usize) -> Word { self.edits[word][index].0 }
}
