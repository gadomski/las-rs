//! Las points.

pub struct Point {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub intensity: u16,
    // TODO these aren't actually u8s
    pub return_number: u8,
    pub number_of_returns: u8,
    pub scan_direction: ScanDirection,
    pub edge_of_flight_line: bool,
    pub classification: Classification,
    pub scan_angle_rank: u8,
    pub user_data: u8,
    pub point_source_id: u16,
}

#[derive(Debug, PartialEq)]
pub enum ScanDirection {
    Backward,
    Forward,
}

#[derive(Debug, PartialEq)]
pub enum Classification {
    CreatedNeverClassified = 0,
    Unclassified = 1,
    Ground = 2,
    LowVegetation = 3,
    MediumVegetation = 4,
    HighVegetation = 5,
    Building = 6,
    LowPoint = 7,
    ModelKeyPoint = 8,
    Water = 9,
    Reserved10 = 10,
    Reserved11 = 11,
    Overlap = 12,
    Reserved,
}
