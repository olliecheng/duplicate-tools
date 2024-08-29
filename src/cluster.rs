use crate::generate_index::Hamming;
use crate::record::Record;

use anyhow::Result;

use bk_tree::{metrics, BKTree, Metric};
use triple_accel::hamming;

use itertools::Itertools;
use murmur3::murmur3_32;
use std::io::Cursor;

struct RecordDist;

fn max_thresh(v: u32, thresh: u32) -> u32 {
    if v > thresh {
        return 100;
    }
    return v;
}

impl Metric<&Record> for RecordDist {
    fn distance(&self, a: &&Record, b: &&Record) -> u32 {
        // out of 20, where 0 is the closest / exact match
        let jaccard: u32 =
            (unsafe { 20_usize.unchecked_sub(a.minhash.intersection(&b.minhash).count()) })
                .try_into()
                .unwrap();

        return jaccard;
        // if a.id.bc.as_bytes().len() != b.id.bc.as_bytes().len() {
        //     println!("a: {:?}, b: {:?}", a.id, b.id);
        // }
        max_thresh(jaccard, 6)
            + max_thresh(hamming(a.id.bc.as_bytes(), b.id.bc.as_bytes()), 2)
            + max_thresh(hamming(a.id.umi.as_bytes(), b.id.umi.as_bytes()), 2)
    }

    fn threshold_distance(&self, a: &&Record, b: &&Record, threshold: u32) -> Option<u32> {
        let v = self.distance(a, b);

        if v <= threshold {
            Some(v)
        } else {
            None
        }
    }
}

pub fn cluster_from(index: &str) -> Result<()> {
    let records: Vec<Record> = bincode::deserialize_from(std::fs::File::open(index)?)?;
    info!("Done retreiving records");

    // generate BKTree
    let mut tree: BKTree<&Record, RecordDist> = BKTree::new(RecordDist);
    for rec in records.iter() {
        if rec.id.bc != "" {
            tree.add(rec);
        }
    }

    info!("Tree done!");

    println!("{:?}", tree.find(&&records[0], 5).collect::<Vec<_>>());

    let mut m = Vec::new();

    for (idx, rec) in records.iter().enumerate() {
        if idx > 100 {
            break;
        }

        let results = tree.find(&rec, 5);

        for (sim, rec2) in results {
            let bc_hamming = hamming(rec.id.bc.as_bytes(), rec2.id.bc.as_bytes());
            if bc_hamming > 2 {
                continue;
            }

            let umi_hamming = hamming(rec.id.umi.as_bytes(), rec2.id.umi.as_bytes());
            if umi_hamming > 3 {
                continue;
            }

            m.push((&rec2.id, sim, bc_hamming, umi_hamming, rec2.loc));
            // m.push((&rec2.id, sim, "test1", "test2", rec2.loc));
        }

        if m.len() > 1 {
            println!("\nOriginal  {:?} {idx}", rec.id);
            for i in m.iter() {
                println!("Discovery {:?}", i);
            }
        }

        m.clear();
    }

    // for (idx, first_rec) in records.iter().enumerate() {
    //     // info!("{:?}", first_rec.id);

    //     for rec in records
    //         .iter()
    //         .enumerate()
    //         .filter_map(|(v, rec)| if v == idx { None } else { Some(rec) })
    //     {
    //         let cnt = first_rec.minhash.intersection(&rec.minhash).count();
    //         if cnt > 10 {
    //             // println!("{cnt}: {:?} at {}", rec.id, rec.loc);
    //         }
    //     }
    // }
    Ok(())
}
