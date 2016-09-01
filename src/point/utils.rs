//! Point-related utility functions.

const MASK: u8 = 0b10000000;

/// Returns true if the u8 indicates that the point is the edge of a flight line.
pub fn edge_of_flight_line(n: u8) -> bool {
    (MASK & n) == MASK
}

/// Returns the u8 mask used to indiate that a point is the edge of a flight line.
pub fn edge_of_flight_line_u8(edge: bool) -> u8 {
    if edge { MASK } else { 0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_of_flight_line_true() {
        assert!(edge_of_flight_line(0b10000000));
    }

    #[test]
    fn edge_of_flight_line_false() {
        assert!(!edge_of_flight_line(0));
    }

    #[test]
    fn edge_of_flight_line_u8_true() {
        assert_eq!(0b10000000, edge_of_flight_line_u8(true));
    }

    #[test]
    fn edge_of_flight_line_u8_false() {
        assert_eq!(0, edge_of_flight_line_u8(false));
    }
}
