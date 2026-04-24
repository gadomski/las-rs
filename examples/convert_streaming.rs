//! Converts between LAS and LAZ (either direction), streaming chunks of
//! points through a reusable [`las::PointData`] byte slab. Memory use is
//! bounded by the chunk size regardless of file size, and no
//! [`las::Point`] is ever materialized on the path from input to output.
//!
//! ```text
//! cargo run --release --example convert_streaming --features laz-parallel -- in.laz out.las
//! cargo run --release --example convert_streaming --features laz-parallel -- in.las out.laz
//! ```
//!
//! This is the middle ground between `convert_bulk.rs` (whole file in
//! RAM) and `convert_point_by_point.rs` (one `Point` at a time): same
//! zero-materialization fast path as bulk, but with a fixed memory
//! footprint. Reach for it when the file is too big to hold as one
//! [`las::PointData`] but you still want the throughput of the byte-slab
//! path.

use las::{PointData, Reader, Writer};
use std::{env, io::Write as _};

/// One LAZ chunk's worth of points, matched to the reader's internal
/// batch so the parallel decompressor stays in its fast path.
const CHUNK: u64 = 500_000;

fn print_progress(seen: u64, total: u64) {
    let pct = if total > 0 {
        (seen as f64 / total as f64) * 100.0
    } else {
        100.0
    };
    eprint!("\rprocessed {seen} / {total} points ({pct:.1}%)");
    let _ = std::io::stderr().flush();
}

fn main() {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .expect("usage: convert_streaming <input> <output>");
    let output = args
        .next()
        .expect("usage: convert_streaming <input> <output>");

    let mut reader = Reader::from_path(&input).expect("open input");
    let header = reader.header().clone();
    let total = header.number_of_points();
    let format = *reader.header().point_format();
    let transforms = *reader.header().transforms();

    // Reusable chunk buffer — grows to CHUNK points once, never beyond.
    let mut pd = PointData::new(format, transforms);
    let mut writer = Writer::from_path(&output, header).expect("open output");
    let mut seen = 0u64;

    loop {
        let n = reader.fill_points(CHUNK, &mut pd).expect("read chunk");
        if n == 0 {
            break;
        }
        seen += n;
        writer.write_points(&pd).expect("write chunk");
        print_progress(seen, total);
    }
    eprintln!();

    writer.close().expect("close writer");
    println!("Wrote {seen} points to {output}");
}
