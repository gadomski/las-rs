//! Converts between LAS and LAZ (either direction), picking compression by
//! the output extension. Loads the entire file into memory.
//!
//! ```text
//! cargo run --release --example convert_bulk --features laz-parallel -- in.laz out.las
//! cargo run --release --example convert_bulk --features laz-parallel -- in.las out.laz
//! ```
//!
//! Companion to `convert_streaming.rs`. This version is simpler and
//! significantly faster when the file comfortably fits in RAM: the
//! [`Reader`] hands a [`las::PointData`] byte slab straight to the
//! [`Writer`], so no [`las::Point`] ever gets materialized on the path
//! from input to output. Switch to the streaming version for files
//! that don't fit.

use las::{Reader, Writer};
use std::env;

fn main() {
    let mut args = env::args().skip(1);
    let input = args.next().expect("usage: convert_bulk <input> <output>");
    let output = args.next().expect("usage: convert_bulk <input> <output>");

    let mut reader = Reader::from_path(&input).expect("open input");
    let header = reader.header().clone();

    // Read the whole file as one PointData byte slab …
    let pd = reader.read_all().expect("read all");
    let n = pd.len();

    // … and hand it to the writer in a single batched call. No per-point
    // decode/encode round-trip.
    let mut writer = Writer::from_path(&output, header).expect("open output");
    writer.write_points(&pd).expect("write points");
    writer.close().expect("close writer");

    println!("Wrote {n} points to {output}");
}
