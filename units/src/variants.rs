use std::fmt::{Debug, Formatter};
use std::hash::Hash;
use std::iter::Once;
use std::marker::PhantomData;
use std::{iter, vec};

use im::{hashmap, hashset};
use itertools::Itertools;
use retain_mut::RetainMut;
use ustr::{ustr, Ustr};

#[derive(Eq, PartialEq, Clone, Hash)]
pub struct Note(Ustr);

#[derive(Eq, PartialEq, Clone, Debug)]
pub struct VariantSet<V: VariantKey>(Vec<im::HashMap<V, im::HashSet<Vec<Note>>>>);

pub struct VariantFn<'a, V: VariantKey>(Box<dyn 'a + FnOnce(&VariantCtx) -> VariantSet<V>>);

pub struct VariantCtx {
    max_notes: usize,
}

pub trait VariantKey = 'static + Eq + Hash + Clone;

impl Note {
    pub fn new(note: Ustr) -> Self { Note(note) }
}

impl<V: VariantKey> VariantSet<V> {
    pub fn iter(&self) -> impl Iterator<Item = (&V, &[Note])> {
        self.0.iter().flat_map(|map| {
            map.iter()
                .flat_map(|(k, vs)| vs.iter().map(move |v| (k, v.as_slice())))
        })
    }
}

impl VariantCtx {
    pub fn new(max_notes: usize) -> Self { VariantCtx { max_notes } }
    pub fn empty<V: VariantKey>(&self) -> VariantSet<V> { VariantSet(vec![]) }
    pub fn push<V: VariantKey>(&self, vs: &mut VariantSet<V>, v: V, notes: Vec<Note>) {
        if notes.len() <= self.max_notes {
            if vs.0.len() <= notes.len() {
                vs.0.resize(notes.len() + 1, hashmap! {});
            }
            vs.0[notes.len()].entry(v).or_default().insert(notes);
        }
    }
    fn push_all<V: VariantKey>(
        &self,
        vs: &mut VariantSet<V>,
        value: V,
        x_note_set: &im::HashSet<Vec<Note>>,
        y_note_set: &im::HashSet<Vec<Note>>,
    ) {
        for x_note in x_note_set.iter() {
            for y_note in y_note_set.iter() {
                let mut xy_note = x_note.clone();
                xy_note.append(&mut y_note.clone());
                self.push(vs, value.clone(), xy_note);
            }
        }
    }
    pub fn correct<V: VariantKey>(&self, v: V) -> VariantSet<V> {
        let mut result = self.empty();
        self.push(&mut result, v, vec![]);
        result
    }
    pub fn all_correct<V: VariantKey>(&self, vs: impl IntoIterator<Item = V>) -> VariantSet<V> {
        let mut result = self.empty();
        for v in vs {
            self.push(&mut result, v, vec![]);
        }
        result
    }
    pub fn incorrect<V: VariantKey>(&self, v: V, note: Note) -> VariantSet<V> {
        let mut result = self.empty();
        self.push(&mut result, v, vec![note]);
        result
    }
    pub fn union<V: VariantKey>(&self, a: &VariantSet<V>, b: &VariantSet<V>) -> VariantSet<V> {
        VariantSet(
            a.0.iter()
                .cloned()
                .zip_longest(b.0.iter().cloned())
                .map(|x| {
                    x.reduce(|a, b| {
                        a.clone()
                            .union_with(b.clone(), |a, b| a.clone().union(b.clone()))
                    })
                })
                .collect(),
        )
    }
    pub fn cross<V1: VariantKey, V2: VariantKey, V3: VariantKey>(
        &self,
        x: &VariantSet<V1>,
        y: &VariantSet<V2>,
        f: impl Fn(&V1, &V2) -> V3,
    ) -> VariantSet<V3> {
        let mut result = self.empty();
        for (x_note_count, x_group) in x.0.iter().enumerate() {
            for (y_note_count, y_group) in
                y.0.iter()
                    .take(1 + self.max_notes - x_note_count)
                    .enumerate()
            {
                for (x_value, x_note_set) in x_group.iter() {
                    for (y_value, y_note_set) in y_group.iter() {
                        self.push_all(&mut result, f(x_value, y_value), x_note_set, y_note_set);
                    }
                }
            }
        }
        result
    }
    pub fn then<V1: VariantKey, V2: VariantKey>(
        &self,
        x: &VariantSet<V1>,
        f: impl Fn(&VariantCtx, &V1) -> VariantSet<V2>,
    ) -> VariantSet<V2> {
        let mut result = self.empty();
        for (note_count, x_group) in x.0.iter().enumerate() {
            let ctx = VariantCtx {
                max_notes: self.max_notes - note_count,
            };
            for (x_value, x_note_set) in x_group.iter() {
                let y = f(&ctx, x_value);
                for y_group in y.0.iter() {
                    for (y_value, y_note_set) in y_group.iter() {
                        self.push_all(&mut result, y_value.clone(), x_note_set, y_note_set);
                    }
                }
            }
        }
        result
    }
}

impl<'a, V: VariantKey> VariantFn<'a, V> {
    pub fn new(f: impl 'a + FnOnce(&VariantCtx) -> VariantSet<V>) -> Self { VariantFn(box f) }
    pub fn resolve(self, ctx: &VariantCtx) -> VariantSet<V> { (self.0)(ctx) }
    pub fn empty() -> Self { Self::new(|ctx| ctx.empty()) }
    pub fn correct(v: V) -> Self { Self::new(|ctx| ctx.correct(v)) }
    pub fn all_correct(vs: impl 'a + IntoIterator<Item = V>) -> Self {
        Self::new(|ctx| ctx.all_correct(vs))
    }
    pub fn incorrect(v: V, note: Note) -> Self { Self::new(|ctx| ctx.incorrect(v, note)) }
    pub fn union(self, other: Self) -> Self {
        Self::new(|ctx| ctx.union(&self.resolve(ctx), &other.resolve(ctx)))
    }
    pub fn then<V2: VariantKey>(
        self,
        f: impl 'a + Fn(V) -> VariantFn<'a, V2>,
    ) -> VariantFn<'a, V2> {
        VariantFn::new(move |ctx| {
            ctx.then(&self.resolve(ctx), |ctx2, v| f(v.clone()).resolve(ctx2))
        })
    }
    pub fn cross<V2: VariantKey, V3: VariantKey>(
        self,
        other: VariantFn<'a, V2>,
        f: impl 'a + Fn(&V, &V2) -> V3,
    ) -> VariantFn<'a, V3> {
        VariantFn::new(move |ctx| ctx.cross(&self.resolve(ctx), &other.resolve(ctx), f))
    }
}

impl Debug for Note {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.0) }
}

// #[derive(Eq, PartialEq, Clone)]
// pub struct VariantItem<V, E> {
//     pub value: V,
//     pub notes: Vec<E>,
// }
//
// pub trait Variants: Iterator<Item=Vec<VariantItem<Self::Value, Self::Note>>> {
//     type Value;
//     type Note;
//     fn then<F, I2>(
//         self,
//         fun: F,
//     ) -> Then<Self, F>
//         where F: FnMut(Self::Value) -> I2,
//               I2: Variants<Note=Self::Note>,
//               Self: Sized,
//               Self::Note: Clone {
//         Then { i1: self, fun, results: vec![] }
//     }
//     fn union<I2>(self, other: I2) -> Union<Self, I2> where I2: Variants<Value=Self::Value, Note=Self::Note>, Self: Sized {
//         Union { i1: self, i2: other }
//     }
// }
//
// impl<V, E, T> Variants for T where T: Iterator<Item=Vec<VariantItem<V, E>>> {
//     type Value = V;
//     type Note = E;
// }
//
// pub struct Then<I1, F> where I1: Variants, F: FnMut<(I1::Value, )>, F::Output: Variants<Note=I1::Note>, I1::Note: Clone {
//     i1: I1,
//     fun: F,
//     results: Vec<(Vec<I1::Note>, F::Output)>,
// }
//
// impl<I1, F, I2> Iterator for Then<I1, F> where I1: Variants, F: FnMut(I1::Value) -> I2, I2: Variants<Note=I1::Note>, I1::Note: Clone {
//     type Item = Vec<VariantItem<I2::Value, I1::Note>>;
//     fn next(&mut self) -> Option<Self::Item> {
//         let end_of_input;
//         if let Some(inputs) = self.i1.next() {
//             end_of_input = false;
//             for input in inputs {
//                 self.results.push((input.notes, (self.fun)(input.value)))
//             }
//         } else {
//             end_of_input = true;
//         }
//         let mut results = vec![];
//         self.results.retain_mut(|(first_notes, iter)| {
//             if let Some(outputs) = iter.next() {
//                 for mut output in outputs {
//                     let mut notes = first_notes.clone();
//                     notes.append(&mut output.notes);
//                     results.push(VariantItem { value: output.value, notes });
//                 }
//                 true
//             } else {
//                 false
//             }
//         });
//         if self.results.is_empty() && end_of_input {
//             None
//         } else {
//             Some(results)
//         }
//     }
// }
//
// #[derive(Clone)]
// pub struct Union<I1, I2> where I1: Variants, I2: Variants<Value=I1::Value, Note=I1::Note> {
//     i1: I1,
//     i2: I2,
// }
//
// impl<I1, I2> Iterator for Union<I1, I2> where I1: Variants, I2: Variants<Value=I1::Value, Note=I1::Note> {
//     type Item = I1::Item;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         match (self.i1.next(), self.i2.next()) {
//             (Some(mut r1), Some(mut r2)) => {
//                 r1.append(&mut r2);
//                 Some(r1)
//             }
//             (None, Some(r2)) => Some(r2),
//             (Some(r1), None) => Some(r1),
//             (None, None) => None,
//         }
//     }
// }
//
// #[derive(Clone)]
// pub struct Correct<V, E>(Once<Vec<VariantItem<V, E>>>);
//
// pub fn correct<V, E>(value: V) -> Correct<V, E> {
//     Correct(iter::once(vec![VariantItem { value, notes: vec![] }]))
// }
//
// impl<V, E> Iterator for Correct<V, E> {
//     type Item = Vec<VariantItem<V, E>>;
//     fn next(&mut self) -> Option<Self::Item> { self.0.next() }
// }
//
// #[derive(Clone)]
// pub struct Incorrect<V, E>(vec::IntoIter<Vec<VariantItem<V, E>>>);
//
// pub fn incorrect<V, E>(value: V, error: E) -> Incorrect<V, E> {
//     Incorrect(vec![vec![], vec![VariantItem { value, notes: vec![error] }]].into_iter())
// }
//
// impl<V, E> Iterator for Incorrect<V, E> {
//     type Item = Vec<VariantItem<V, E>>;
//     fn next(&mut self) -> Option<Self::Item> { self.0.next() }
// }
//
// pub struct ErrorItemIntoIter<V, E> {
//     head: usize,
//     tail: Option<Vec<VariantItem<V, E>>>,
// }
//
// impl<V, E> IntoIterator for VariantItem<V, E> {
//     type Item = Vec<VariantItem<V, E>>;
//     type IntoIter = ErrorItemIntoIter<V, E>;
//
//     fn into_iter(self) -> Self::IntoIter {
//         ErrorItemIntoIter { head: self.notes.len(), tail: Some(vec![self]) }
//     }
// }
//
// impl<V, E> Iterator for ErrorItemIntoIter<V, E> {
//     type Item = Vec<VariantItem<V, E>>;
//
//     fn next(&mut self) -> Option<Self::Item> {
//         if self.head > 0 {
//             self.head -= 1;
//             Some(vec![])
//         } else {
//             self.tail.take()
//         }
//     }
// }
//
// impl<V: Debug, E: Debug> Debug for VariantItem<V, E> {
//     fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{:?}", self.value)?;
//         if !self.notes.is_empty() {
//             write!(f, "{:?}", self.notes)?;
//         }
//         Ok(())
//     }
// }
//
// impl<V, E> VariantItem<V, E> {
//     pub fn new(value: V, notes: Vec<E>) -> Self { VariantItem { value, notes } }
// }
//
#[test]
fn test_errors() {
    let error1 = Note::new(ustr("error1"));
    let error2 = Note::new(ustr("error2"));
    let output_fn = || {
        let input = VariantFn::correct(9);
        input.then(|x| {
            let feet_per_yard =
                VariantFn::correct(3).union(VariantFn::incorrect(1, error1.clone()));
            let inches_per_foot =
                VariantFn::correct(12).union(VariantFn::incorrect(1, error2.clone()));
            feet_per_yard.cross(inches_per_foot, move |y, z| x * y * z)
        })
    };
    let expected = vec![
        hashmap! {
            324 => hashset![vec![]]
        },
        hashmap! {
            27 => hashset![vec![error2.clone()]],
            108 => hashset![vec![error1.clone()]],
        },
        hashmap! {
            9 => hashset![vec![error1.clone(), error2.clone()]],
        },
    ];
    for max_notes in 0..=2 {
        let output = output_fn().resolve(&VariantCtx::new(max_notes));
        println!("{:?}", output);
        assert_eq!(
            output,
            VariantSet(expected[0..=max_notes].iter().cloned().collect())
        );
    }
}
