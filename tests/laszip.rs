extern crate las;

use las::Reader;

#[test]
fn detect_laszip() {
    assert!(Reader::from_path("tests/data/autzen.laz").is_err());
}
