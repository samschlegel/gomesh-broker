use phf::phf_set;

/// Set of valid IATA airport codes.
///
/// This is a subset of commonly used codes. In production this could be
/// loaded from a data file or expanded to the full IATA list.
static VALID_IATA_CODES: phf::Set<&'static str> = phf_set! {
    // North America
    "ATL", "LAX", "ORD", "DFW", "DEN", "JFK", "SFO", "SEA", "LAS", "MCO",
    "EWR", "MSP", "BOS", "DTW", "PHL", "LGA", "FLL", "BWI", "SLC", "SAN",
    "IAD", "DCA", "MDW", "TPA", "PDX", "HNL", "STL", "MCI", "RDU", "AUS",
    "CLE", "SMF", "SJC", "SAT", "IND", "PIT", "CMH", "MKE", "OAK", "RSW",
    "BNA", "CVG", "MSY", "JAX", "ONT", "SNA", "BUR", "ABQ", "OMA", "MEM",
    "YYZ", "YVR", "YUL", "YYC", "YOW", "YEG", "YHZ", "MEX", "CUN", "GDL",
    // Europe
    "LHR", "CDG", "FRA", "AMS", "MAD", "BCN", "FCO", "MUC", "IST", "ZRH",
    "OSL", "CPH", "VIE", "BRU", "DUB", "LIS", "HEL", "WAW", "PRG", "BUD",
    "ATH", "ARN", "MAN", "STN", "LGW", "EDI", "GVA", "NCE", "HAM", "TXL",
    // Asia-Pacific
    "HND", "NRT", "PEK", "PVG", "HKG", "SIN", "ICN", "BKK", "KUL", "SYD",
    "MEL", "AKL", "DEL", "BOM", "TPE", "MNL", "CGK", "KIX", "CTS", "FUK",
    // Middle East / Africa
    "DXB", "DOH", "AUH", "JED", "RUH", "TLV", "CAI", "JNB", "CPT", "NBO",
    // South America
    "GRU", "GIG", "EZE", "BOG", "SCL", "LIM", "PTY", "MVD", "BSB", "CNF",
};

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
    }

    #[test]
    fn invalid_codes() {
        assert!(!is_valid_iata("ZZZ"));
        assert!(!is_valid_iata(""));
        assert!(!is_valid_iata("lax")); // Case-sensitive
    }
}
