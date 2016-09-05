
const SYNTHETIC_MASK: u8 = 0b00100000;
const KEY_POINT_MASK: u8 = 0b01000000;
const WITHHELD_MASK: u8 = 0b10000000;
const ASPRS_CLASSIFICATION_MASK: u8 = 0b00011111;

/// Point classification.
///
/// In version 1.0, this was a user-defined and optional u8. In subsequent versions, this field was
/// defined more rigidly.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Classification {
    /// The ASPRS type classification.
    classification: ASPRSClassification,
    /// True if this point was created via sythetic means, such as through photogrammetry.
    synthetic: bool,
    /// True if this is a model keypoint and should not be removed by future thinning.
    key_point: bool,
    /// True if this point should be excluded from processing.
    withheld: bool,
}

impl From<u8> for Classification {
    fn from(n: u8) -> Classification {
        Classification {
            classification: ASPRSClassification::from(n),
            synthetic: (n & SYNTHETIC_MASK) == SYNTHETIC_MASK,
            key_point: (n & KEY_POINT_MASK) == KEY_POINT_MASK,
            withheld: (n & WITHHELD_MASK) == WITHHELD_MASK,
        }
    }
}

impl From<Classification> for u8 {
    fn from(classification: Classification) -> u8 {
        let mut n = u8::from(classification.classification);
        if classification.synthetic {
            n += SYNTHETIC_MASK;
        }
        if classification.key_point {
            n += KEY_POINT_MASK;
        }
        if classification.withheld {
            n += WITHHELD_MASK;
        }
        n
    }
}

impl PartialEq<Classification> for u8 {
    fn eq(&self, other: &Classification) -> bool {
        *self == u8::from(*other)
    }
}

/// ASPRS classification table.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ASPRSClassification {
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

impl From<u8> for ASPRSClassification {
    fn from(n: u8) -> ASPRSClassification {
        match n & ASPRS_CLASSIFICATION_MASK {
            0 => ASPRSClassification::CreatedNeverClassified,
            1 => ASPRSClassification::Unclassified,
            2 => ASPRSClassification::Ground,
            3 => ASPRSClassification::LowVegetation,
            4 => ASPRSClassification::MediumVegetation,
            5 => ASPRSClassification::HighVegetation,
            6 => ASPRSClassification::Building,
            7 => ASPRSClassification::LowPoint,
            8 => ASPRSClassification::ModelKeyPoint,
            9 => ASPRSClassification::Water,
            12 => ASPRSClassification::OverlapPoints,
            _ => ASPRSClassification::Reserved(n),
        }
    }
}

impl From<ASPRSClassification> for u8 {
    fn from(classification: ASPRSClassification) -> u8 {
        match classification {
            ASPRSClassification::CreatedNeverClassified => 0,
            ASPRSClassification::Unclassified => 1,
            ASPRSClassification::Ground => 2,
            ASPRSClassification::LowVegetation => 3,
            ASPRSClassification::MediumVegetation => 4,
            ASPRSClassification::HighVegetation => 5,
            ASPRSClassification::Building => 6,
            ASPRSClassification::LowPoint => 7,
            ASPRSClassification::ModelKeyPoint => 8,
            ASPRSClassification::Water => 9,
            ASPRSClassification::OverlapPoints => 12,
            ASPRSClassification::Reserved(n) => n,
        }
    }
}

impl Default for ASPRSClassification {
    fn default() -> ASPRSClassification {
        ASPRSClassification::CreatedNeverClassified
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_from() {
        fn classification(s: bool, k: bool, w: bool) -> Classification {
            Classification {
                classification: ASPRSClassification::Ground,
                synthetic: s,
                key_point: k,
                withheld: w,
            }
        };
        assert_eq!(classification(false, false, false), Classification::from(2));
        assert_eq!(classification(true, false, false),
                   Classification::from(0b00100010));
        assert_eq!(classification(false, true, false),
                   Classification::from(0b01000010));
        assert_eq!(classification(false, false, true),
                   Classification::from(0b10000010));
    }
}
