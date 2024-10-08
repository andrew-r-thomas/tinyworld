use std::{
    cmp::Reverse,
    collections::{BTreeMap, BinaryHeap},
    fs::File,
    io::{BufRead, BufReader, LineWriter, Write},
};

use ordered_float::OrderedFloat;
use parquet::format::XxHash;
use serde::{Deserialize, Serialize};
use sorted_vec::partial::{ReverseSortedVec, SortedVec};
use tinyworld::distance_calculators::{DistanceCalculator, SimpleDotProduct};

#[derive(Serialize, Deserialize, Debug)]
struct TestInData {
    word: String,
    emb: Vec<f32>,
}

#[derive(Deserialize, Serialize, Debug)]
struct TestOutData {
    word: String,
    emb: Vec<f32>,
    matches: Vec<String>,
}

struct WordDist {
    word: String,
    dist: f32,
}

impl PartialOrd for WordDist {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.dist.partial_cmp(&other.dist)
    }
}

impl PartialEq for WordDist {
    fn eq(&self, other: &Self) -> bool {
        self.dist.eq(&other.dist)
    }
}

fn main() {
    let mut data = {
        let file = File::open("test_data.json").unwrap();
        let rdr = BufReader::new(file);
        rdr.lines()
            .map(|l| serde_json::from_str::<TestInData>(&l.unwrap()).unwrap())
            .collect::<Vec<TestInData>>()
    };

    let out_file = File::create("test_data_temp.json").unwrap();
    let mut writer = LineWriter::new(out_file);
    let mut calc = SimpleDotProduct {};
    for datum in &data {
        let mut dists = ReverseSortedVec::new();

        for d in &data {
            let dist = calc.calc_dist(&datum.emb, &d.emb);
            dists.insert(WordDist {
                word: d.word.clone(),
                dist,
            });
        }

        println!(
            "word: {}, top_match: {}, dist: {}, matches_len: {}",
            datum.word,
            dists.first().unwrap().dist,
            dists.first().unwrap().word,
            dists.len()
        );

        let mut l = serde_json::to_string(&TestOutData {
            word: datum.word.clone(),
            emb: datum.emb.clone(),
            matches: dists.iter().map(|x| x.word.clone()).collect(),
        })
        .unwrap();
        l.push_str("\n");

        writer.write(l.as_bytes()).unwrap();

        // ms.push(dist_map.into_values().collect::<Vec<String>>());
    }

    // for (d, m) in data.iter_mut().zip(ms) {
    //     d.matches = Some(m);
    // }

    // println!(
    //     "word: {}, matches: {:?}",
    //     data[0].word,
    //     &data[0].matches.as_ref().unwrap()[0..3]
    // );
}
