use crate::{point::Error, Result};

/// The ASPRS classification table.
///
/// Classifications can be created from u8s and converted back into them:
///
/// ```
/// use las::point::Classification;
/// let classification = Classification::new(2).unwrap();
/// assert_eq!(Classification::Ground, classification);
/// assert_eq!(2, u8::from(classification));
/// ```
///
/// We make one modification to this table: we remove `OverlapPoints`, code 12. Las 1.4 added the
/// extended point formats, which include an overlap bit. The overlap bit is intended to allow a
/// point to both be an overlap point and contain some other classification.
///
/// Here's how we deal with that change:
///
/// - If the point format doesn't support the overlap bit, the classification is overwritten with
///   the code for overlap points (12). On ingest, points with an overlap classification are given
///   the `Unclassified` code and `Point::is_overlap` is set to `true`.
/// - If the point format does support the overlap bit, that is preferred.
///
/// Because of this change, trying to create a classification with code 12 is an error:
///
/// ```
/// use las::point::Classification;
/// assert!(Classification::new(12).is_err());
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[allow(missing_docs)]
pub enum Classification {
    #[default]
    CreatedNeverClassified,
    Unclassified,
    Ground,
    LowVegetation,
    MediumVegetation,
    HighVegetation,
    Building,
    LowPoint,
    ModelKeyPoint,
    Water,
    Rail,
    RoadSurface,
    WireGuard,
    WireConductor,
    TransmissionTower,
    WireStructureConnector,
    BridgeDeck,
    HighNoise,
    Reserved(u8),
    UserDefinable(u8),
}

impl Classification {
    /// Creates a new classification.
    ///
    /// Throws an error if the classification is 12 (overlap).
    ///
    /// # Examples
    ///
    /// ```
    /// use las::point::Classification;
    /// assert_eq!(Classification::Ground, Classification::new(2).unwrap());
    /// assert!(Classification::new(12).is_err());
    /// ```
    pub fn new(n: u8) -> Result<Classification> {
        Ok(match n {
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
            10 => Classification::Rail,
            11 => Classification::RoadSurface,
            12 => return Err(Error::OverlapClassification.into()),
            13 => Classification::WireGuard,
            14 => Classification::WireConductor,
            15 => Classification::TransmissionTower,
            16 => Classification::WireStructureConnector,
            17 => Classification::BridgeDeck,
            18 => Classification::HighNoise,
            19..=63 => Classification::Reserved(n),
            64..=255 => Classification::UserDefinable(n),
        })
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
            Classification::Rail => 10,
            Classification::RoadSurface => 11,
            Classification::WireGuard => 13,
            Classification::WireConductor => 14,
            Classification::TransmissionTower => 15,
            Classification::WireStructureConnector => 16,
            Classification::BridgeDeck => 17,
            Classification::HighNoise => 18,
            Classification::Reserved(n) | Classification::UserDefinable(n) => n,
        }
    }
}
