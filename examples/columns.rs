//! Computes column-oriented statistics over a LAS/LAZ file without
//! materializing any [`las::Point`] values.
//!
//! ```text
//! cargo run --release --example columns --features laz-parallel -- tile.laz
//! ```
//!
//! Demonstrates the byte-slab [`las::PointData`] API:
//!   - bulk bounding box over the x/y/z columns (three scalar reductions),
//!   - intensity min/max/mean (one scan),
//!   - per-class histogram of the classification column.
//!
//! Compared with the per-[`las::Point`] API this skips the full point
//! decode on every record and only pays for the fields it touches — the
//! throughput win grows with file size.

use las::Reader;
use std::{collections::BTreeMap, env};

fn main() {
    let input = env::args()
        .nth(1)
        .expect("usage: columns <input.las|laz>");

    let mut reader = Reader::from_path(&input).expect("open input");
    let total = reader.header().number_of_points();
    let points = reader.read_all().expect("read all");
    assert_eq!(points.len() as u64, total);

    // Bounding box — three column passes, one scalar out of each.
    let (min_x, max_x) = points
        .x()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    let (min_y, max_y) = points
        .y()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });
    let (min_z, max_z) = points
        .z()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
            (lo.min(v), hi.max(v))
        });

    // Intensity stats — min/max/mean in a single pass.
    let (i_min, i_max, i_sum, i_n) = points.intensity().fold(
        (u16::MAX, u16::MIN, 0u64, 0u64),
        |(lo, hi, sum, n), v| (lo.min(v), hi.max(v), sum + v as u64, n + 1),
    );
    let i_mean = if i_n > 0 { i_sum as f64 / i_n as f64 } else { 0.0 };

    // Classification histogram — one pass over the classification column.
    let mut hist: BTreeMap<u8, u64> = BTreeMap::new();
    for class in points.classification() {
        *hist.entry(class).or_insert(0) += 1;
    }

    println!("points:       {}", points.len());
    println!(
        "bbox x:       [{:.3}, {:.3}] ({:.3} wide)",
        min_x,
        max_x,
        max_x - min_x
    );
    println!(
        "bbox y:       [{:.3}, {:.3}] ({:.3} wide)",
        min_y,
        max_y,
        max_y - min_y
    );
    println!(
        "bbox z:       [{:.3}, {:.3}] ({:.3} wide)",
        min_z,
        max_z,
        max_z - min_z
    );
    println!(
        "intensity:    min={} max={} mean={:.1}",
        i_min, i_max, i_mean
    );
    println!("classification histogram:");
    for (class, count) in &hist {
        println!("  {:3}: {}", class, count);
    }
}
