/// ASPRS classification table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Classification {
    /// Created, never classified.
    CreatedNeverClassified,
    /// Unclassified.
    Unclassified,
    /// Ground.
    Ground,
    /// Low vegetation.
    LowVegetation,
    /// Medium vegetation.
    MediumVegetation,
    /// High vegetation.
    HighVegetation,
    /// Building.
    Building,
    /// Low point (noise).
    LowPoint,
    /// Model key-point (mass point).
    ModelKeyPoint,
    /// Water.
    Water,
    /// Reserved for ASPRS definition.
    Reserved(u8),
    /// Overlap points.
    OverlapPoints,
}

impl From<u8> for Classification {
    fn from(n: u8) -> Classification {
        match n {
            0 => Classification::CreatedNeverClassified,
            1 => Classification::Unclassified,
            2 => Classification::Ground,
            3 => Classification::LowVegetation,
            4 => Classification::MediumVegetation,
            5 => Classification::HighVegetation,
            6 => Classification::Building,
            7 => Classification::LowPoint,
            8 => Classification::ModelKeyPoint,
            9 => Classification::Water,
            12 => Classification::OverlapPoints,
            _ => Classification::Reserved(n),
        }
    }
}

impl From<Classification> for u8 {
    fn from(classification: Classification) -> u8 {
        match classification {
            Classification::CreatedNeverClassified => 0,
            Classification::Unclassified => 1,
            Classification::Ground => 2,
            Classification::LowVegetation => 3,
            Classification::MediumVegetation => 4,
            Classification::HighVegetation => 5,
            Classification::Building => 6,
            Classification::LowPoint => 7,
            Classification::ModelKeyPoint => 8,
            Classification::Water => 9,
            Classification::OverlapPoints => 12,
            Classification::Reserved(n) => n,
        }
    }
}

impl Default for Classification {
    fn default() -> Classification {
        Classification::CreatedNeverClassified
    }
}
