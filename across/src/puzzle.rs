use std::collections::BTreeMap;
use std::fmt::{Debug, Formatter};
use std::{fmt, fs};

use arrayvec::ArrayVec;
use itertools::{multizip, Itertools};

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Cell {
    pub cell: usize,
    pub correct: char,
    pub guess: Option<char>,
    pub variable: Option<usize>,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Variable {
    pub variable: usize,
    pub cell: usize,
    pub letter: char,
    pub offset: usize,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct Clue {
    pub letter: char,
    pub variables: Vec<usize>,
    pub clue: String,
    pub pretty_answer: String,
}

#[derive(Eq, Ord, PartialEq, PartialOrd, Hash, Debug)]
pub struct Puzzle {
    pub cells: Vec<Cell>,
    pub variables: BTreeMap<usize, Variable>,
    pub clues: BTreeMap<char, Clue>,
}

impl Puzzle {
    pub fn parse(filename: &str) -> Self {
        let data = fs::read_to_string(filename).unwrap();
        let mut lines = data.split('\n');
        let [cell_text, clue_texts, meta, pretty_quote, pretty_author, pretty_title, pretty_answers]: [&str; 7] =
            data.split('\n').collect::<ArrayVec<_>>().into_inner().unwrap();
        let mut clues = BTreeMap::new();
        for (letter, clue_text, pretty_answer) in
            multizip(('A'.., clue_texts.split('|'), pretty_answers.split('|')))
        {
            clues.insert(
                letter,
                Clue {
                    letter,
                    variables: vec![],
                    clue: clue_text.to_string(),
                    pretty_answer: pretty_answer.to_string(),
                },
            );
        }
        let mut variable = 0;
        let mut cells = vec![];
        let mut variables = BTreeMap::new();
        let cell_answers = &cell_text[..cell_text.len() / 3];
        let cell_letters = &cell_text[cell_text.len() / 3..2 * cell_text.len() / 3];
        let cell_offsets = &cell_text[2 * cell_text.len() / 3..];
        for (answer, letter, offset) in multizip((
            cell_answers.chars(),
            cell_letters.chars(),
            cell_offsets.chars(),
        )) {
            if answer.is_ascii_alphanumeric() {
                variable += 1;
                let offset = offset as usize - 'A' as usize;
                let cvars = &mut clues.get_mut(&letter).unwrap().variables;
                if cvars.len() <= offset {
                    cvars.resize(offset + 1, 0);
                }
                cvars[offset] = variable;
                variables.insert(
                    variable,
                    Variable {
                        variable,
                        cell: cells.len(),
                        letter,
                        offset,
                    },
                );
                cells.push(Cell {
                    cell: cells.len(),
                    correct: answer,
                    guess: None,
                    variable: Some(variable),
                });
            } else {
                cells.push(Cell {
                    cell: cells.len(),
                    correct: answer,
                    guess: Some(answer),
                    variable: None,
                });
            }
        }
        Puzzle {
            cells,
            variables,
            clues,
        }
    }
}

impl Debug for Cell {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.correct)?;
        if let Some(guess) = self.guess {
            write!(f, "[{}]", guess)?;
        }
        write!(f, " ")?;
        if let Some(variable) = self.variable {
            write!(f, "{} ", variable)?;
        }
        Ok(())
    }
}

impl Debug for Variable {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}[{}]", self.cell, self.letter, self.offset)
    }
}
