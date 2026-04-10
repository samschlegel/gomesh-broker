// Generated at build time from data/iata_codes.txt (source: OurAirports).
include!(concat!(env!("OUT_DIR"), "/iata_set.rs"));

/// Check whether an IATA code is in the known-valid set.
pub fn is_valid_iata(code: &str) -> bool {
    VALID_IATA_CODES.contains(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_codes() {
        assert!(is_valid_iata("LAX"));
        assert!(is_valid_iata("LHR"));
        assert!(is_valid_iata("NRT"));
        assert!(is_valid_iata("MRY")); // Monterey — was missing from old hardcoded list
    }

    #[test]
    fn invalid_codes() {
        assert!(!is_valid_iata("ZZZ"));
        assert!(!is_valid_iata(""));
        assert!(!is_valid_iata("lax")); // Case-sensitive
    }
}
