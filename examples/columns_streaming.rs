//! Same column-oriented statistics as `columns.rs`, but streamed in
//! fixed-size chunks so memory stays bounded regardless of file size.
//!
//! ```text
//! cargo run --release --example columns_streaming --features laz-parallel -- tile.laz
//! ```
//!
//! Reach for this pattern when the input is too big to fit in RAM as a
//! single [`las::PointData`]. The chunk buffer is reused across iterations
//! via [`las::Reader::fill_points`], so only `CHUNK` points' worth of
//! bytes is alive at any moment (plus the small running accumulators).
//!
//! Compared with `columns.rs`, this version trades a few extra lines of
//! plumbing — per-chunk folds that compose into running totals — for a
//! memory footprint that doesn't scale with the file size.
//!
//! Note that not every statistic composes as trivially across chunks:
//! `min`/`max` fold naturally, but a running mean needs a `(sum, count)`
//! pair, not a per-chunk mean.

use las::{PointData, Reader};
use std::{collections::BTreeMap, env, io::Write};

/// One LAZ chunk's worth of points, matched to the reader's internal
/// batch so the parallel decompressor stays in its fast path.
const CHUNK: u64 = 500_000;

#[derive(Debug)]
struct BBox {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,
    min_z: f64,
    max_z: f64,
}

impl BBox {
    fn empty() -> Self {
        Self {
            min_x: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            min_y: f64::INFINITY,
            max_y: f64::NEG_INFINITY,
            min_z: f64::INFINITY,
            max_z: f64::NEG_INFINITY,
        }
    }

    fn update_from(&mut self, pd: &PointData) {
        for v in pd.x() {
            self.min_x = self.min_x.min(v);
            self.max_x = self.max_x.max(v);
        }
        for v in pd.y() {
            self.min_y = self.min_y.min(v);
            self.max_y = self.max_y.max(v);
        }
        for v in pd.z() {
            self.min_z = self.min_z.min(v);
            self.max_z = self.max_z.max(v);
        }
    }
}

#[derive(Debug)]
struct IntensityStats {
    min: u16,
    max: u16,
    /// Running sum and count — a per-chunk mean can't be composed into
    /// a file-level mean without weighting by the chunk size, so we keep
    /// the primitives and divide once at the end.
    sum: u64,
    count: u64,
}

impl IntensityStats {
    fn empty() -> Self {
        Self {
            min: u16::MAX,
            max: u16::MIN,
            sum: 0,
            count: 0,
        }
    }

    fn update_from(&mut self, pd: &PointData) {
        for v in pd.intensity() {
            self.min = self.min.min(v);
            self.max = self.max.max(v);
            self.sum += v as u64;
            self.count += 1;
        }
    }

    fn mean(&self) -> f64 {
        if self.count > 0 {
            self.sum as f64 / self.count as f64
        } else {
            0.0
        }
    }
}

fn print_progress(seen: u64, total: u64) {
    // `\r` + stderr flush keeps the progress line in place without
    // racing with the final result lines printed to stdout.
    let pct = if total > 0 {
        (seen as f64 / total as f64) * 100.0
    } else {
        100.0
    };
    eprint!("\rprocessed {seen} / {total} points ({pct:.1}%)");
    let _ = std::io::stderr().flush();
}

fn main() {
    let input = env::args()
        .nth(1)
        .expect("usage: columns_streaming <input.las|laz>");

    let mut reader = Reader::from_path(&input).expect("open input");
    let total = reader.header().number_of_points();
    let format = *reader.header().point_format();
    let transforms = *reader.header().transforms();

    // Reusable chunk buffer — grows to CHUNK points once, never beyond.
    let mut pd = PointData::new(format, transforms);

    let mut bbox = BBox::empty();
    let mut intensity = IntensityStats::empty();
    let mut hist: BTreeMap<u8, u64> = BTreeMap::new();
    let mut seen = 0u64;

    loop {
        let n = reader.fill_points(CHUNK, &mut pd).expect("read chunk");
        if n == 0 {
            break;
        }
        seen += n;

        bbox.update_from(&pd);
        intensity.update_from(&pd);
        for class in pd.classification() {
            *hist.entry(class).or_insert(0) += 1;
        }

        print_progress(seen, total);
    }
    eprintln!();
    assert_eq!(seen, total);

    println!("points:       {}", seen);
    println!(
        "bbox x:       [{:.3}, {:.3}] ({:.3} wide)",
        bbox.min_x,
        bbox.max_x,
        bbox.max_x - bbox.min_x
    );
    println!(
        "bbox y:       [{:.3}, {:.3}] ({:.3} wide)",
        bbox.min_y,
        bbox.max_y,
        bbox.max_y - bbox.min_y
    );
    println!(
        "bbox z:       [{:.3}, {:.3}] ({:.3} wide)",
        bbox.min_z,
        bbox.max_z,
        bbox.max_z - bbox.min_z
    );
    println!(
        "intensity:    min={} max={} mean={:.1}",
        intensity.min,
        intensity.max,
        intensity.mean()
    );
    println!("classification histogram:");
    for (class, count) in &hist {
        println!("  {:3}: {}", class, count);
    }
}
