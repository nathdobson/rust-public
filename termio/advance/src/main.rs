#![feature(option_result_contains)]
#![allow(unused_imports, unused_variables)]

use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::{stdin, BufRead, BufReader, Error};
use std::sync::Arc;
use std::{iter, str, thread};

use byteorder::ReadBytesExt;
use font::{Font, Glyph, Segment};
use itertools::Itertools;
use unic_bidi::bidi_class::CharBidiClass;
use unic_bidi::BidiClass;
use unic_segment::Graphemes;
use unic_ucd_age::CharAge;
use unic_ucd_segment::grapheme_cluster_break::GraphemeClusterBreak;

fn main() {
    //let font = Font::open("/Users/nathan/Downloads/Menlo-Regular-01.ttf").unwrap();
    let mut graphemes = vec![];
    let chars = ('\u{0021}'..'\u{007E}')
        .chain('\u{00A0}'..='\u{2FFFD}')
        .chain('\u{30000}'..='\u{3FFFF}');
    let mut gcb_sets = HashMap::new();
    for c in chars.clone() {
        gcb_sets
            .entry(GraphemeClusterBreak::of(c))
            .or_insert(vec![])
            .push(c);
    }
    for (gcb, cs) in gcb_sets.iter() {
        let mut iter = cs.iter().peekable();
        let mut ranges = vec![];
        while let Some(&start) = iter.next() {
            let mut end = start as usize;
            //println!("{:?} {:?}", iter.peek(), end);
            while iter.peek().map(|&&x| x as usize).contains(&(end + 1)) {
                end = *iter.next().unwrap() as usize;
            }
            ranges.push(start as usize..=end);
        }
        println!("{:?} {:?}", gcb, ranges);
    }
    let mut patterns = vec![];
    for len in 3..4 {
        for pattern in iter::repeat(gcb_sets.iter())
            .take(len)
            .multi_cartesian_product()
        {
            let string = pattern.iter().map(|(gcb, cs)| cs.first().unwrap()).join("");
            let count = Graphemes::new(&string).count();
            if count == 1 {
                patterns.push(pattern.iter().map(|(gcb, cs)| *gcb).collect::<Vec<_>>());
            }
        }
    }
    println!("{:?}", patterns);
    return;

    let mut str = String::new();
    let mut table = HashMap::new();
    for x in chars.clone() {
        eprintln!("{}", x as usize);
        for y in chars.clone() {
            str.clear();
            str.push(x);
            str.push(y);
            let count = Graphemes::new(&str).count();
            table
                .entry((GraphemeClusterBreak::of(x), GraphemeClusterBreak::of(y)))
                .or_insert(HashSet::new())
                .insert(count);
        }
    }
    println!(
        "counts = {:?}",
        table.values().map(|x| x.len()).collect::<HashSet<_>>()
    );
    for x in '\u{FF}'..='\u{3FFFF}' {
        // let age = x.age();
        // if age.is_none() {
        //     continue;
        // }
        // let class = x.bidi_class();
        // match x.bidi_class() {
        //     BidiClass::ArabicLetter => continue,
        //     BidiClass::ArabicNumber => continue,
        //     BidiClass::ParagraphSeparator => {}
        //     BidiClass::BoundaryNeutral => {}
        //     BidiClass::CommonSeparator => {}
        //     BidiClass::EuropeanNumber => {}
        //     BidiClass::EuropeanSeparator => {}
        //     BidiClass::EuropeanTerminator => {}
        //     BidiClass::FirstStrongIsolate => {}
        //     BidiClass::LeftToRight => {}
        //     BidiClass::LeftToRightEmbedding => {}
        //     BidiClass::LeftToRightIsolate => {}
        //     BidiClass::LeftToRightOverride => {}
        //     BidiClass::NonspacingMark => {}
        //     BidiClass::OtherNeutral => {}
        //     BidiClass::PopDirectionalFormat => {}
        //     BidiClass::PopDirectionalIsolate => {}
        //     BidiClass::RightToLeft => continue,
        //     BidiClass::RightToLeftEmbedding => continue,
        //     BidiClass::RightToLeftIsolate => continue,
        //     BidiClass::RightToLeftOverride => continue,
        //     BidiClass::SegmentSeparator => {}
        //     BidiClass::WhiteSpace => {}
        // }
        // match font.draw(x) {
        //     Ok(Some(g)) => {}
        //     Ok(None) => {
        //         eprintln!("No glyph {:?}", x);
        //         continue;
        //     }
        //     Err(e) => {
        //         eprintln!("{:?} {}", x, e);
        //         continue;
        //     }
        // }
        graphemes.push(format!("{}", x));
    }
    let graphemes = Arc::new(graphemes);
    let writer = thread::spawn({
        let graphemes = graphemes.clone();
        move || {
            for s in graphemes.iter() {
                let points = s
                    .chars()
                    .map(|x| format!("\\u{{{:05X}}}", x as usize))
                    .join("");
                println!("{}\x1B[6n {}\r", s, points);
            }
        }
    });
    let reader = thread::spawn({
        let graphemes = graphemes.clone();
        move || {
            let mut stdin = BufReader::with_capacity(8, stdin());
            let mut advances = BTreeMap::new();
            for g in graphemes.iter() {
                let mut toss = vec![];
                let mut x = vec![];
                let mut y = vec![];
                stdin.read_until(b'\x1B', &mut toss).unwrap();
                let mut empty = vec![];
                stdin.read_until(b'[', &mut empty).unwrap();
                if empty.len() > 1 {
                    eprintln!("empty {:?}", str::from_utf8(&empty));
                    continue;
                }
                stdin.read_until(b';', &mut y).unwrap();
                y.pop();
                stdin.read_until(b'R', &mut x).unwrap();
                x.pop();
                let x = str::from_utf8(&x).unwrap().parse::<usize>().unwrap();
                let y = str::from_utf8(&y).unwrap().parse::<usize>().unwrap();
                advances.insert(g.clone(), x - 1);
            }
            advances
        }
    });
    writer.join().unwrap();
    let advances = reader.join().unwrap();
    advances
        .iter()
        .group_by(|(_, a)| **a)
        .into_iter()
        .for_each(|(advance, cs)| {
            let cs: Vec<_> = cs.collect();
            eprintln!(
                "{:?} {:?}-{:?} [{:?}]",
                advance,
                cs.first().unwrap(),
                cs.last().unwrap(),
                cs.len()
            );
        });
}
