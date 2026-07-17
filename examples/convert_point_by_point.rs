//! Converts between LAS and LAZ by materializing every [`las::Point`]
//! explicitly — the simplest possible conversion program. Loads the
//! entire file into memory.
//!
//! ```text
//! cargo run --release --example convert_point_by_point --features laz-parallel -- in.laz out.las
//! cargo run --release --example convert_point_by_point --features laz-parallel -- in.las out.laz
//! ```
//!
//! Use this pattern when you want to inspect or transform each
//! [`las::Point`] on the way through (filter by classification, edit a
//! field, etc.). For copy-style conversions that don't need access to
//! individual fields, see `convert_bulk.rs` (zero-decode whole-file)
//! or `convert_streaming.rs` (zero-decode chunked) — both skip the
//! [`las::Point`] materialization entirely.

use las::{Reader, Writer};
use std::env;

fn main() {
    let mut args = env::args().skip(1);
    let input = args
        .next()
        .expect("usage: convert_point_by_point <input> <output>");
    let output = args
        .next()
        .expect("usage: convert_point_by_point <input> <output>");

    let mut reader = Reader::from_path(&input).expect("open input");
    let header = reader.header().clone();
    let mut writer = Writer::from_path(&output, header).expect("open output");

    let mut n: u64 = 0;
    for point in reader.read_all().expect("read all").points() {
        writer
            .write_point(point.expect("decode point"))
            .expect("write point");
        n += 1;
    }
    writer.close().expect("close writer");

    println!("Wrote {n} points to {output}");
}
