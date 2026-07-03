use crate::types::TopicParts;

/// The root namespace segment for all MeshCore data topics.
///
/// Real topics are `meshcore/{iata}/{pubkey}/{subtopic...}`. Internal topics
/// (`$SYS/*`) and topics from other applications live under different roots, so
/// confining a subscription filter to this root scopes a subscriber to MeshCore
/// data only.
pub const MESHCORE_ROOT: &str = "meshcore";

/// Whether a subscription topic filter is confined to the [`MESHCORE_ROOT`] namespace.
///
/// Returns `true` only if the filter's first `/`-segment is the literal
/// `meshcore` root. A leading wildcard (`#` or `+`) is rejected because it could
/// match topics outside the namespace (e.g. other applications' roots). `$SYS`
/// topics are excluded for the same reason.
///
/// Examples that return `true`: `meshcore/#`, `meshcore/SEA/#`,
/// `meshcore/SEA/abcd/packets`, `meshcore`.
/// Examples that return `false`: `#`, `+/SEA/#`, `$SYS/#`, `us/LAX/abcd`.
pub fn filter_within_meshcore_root(filter: &str) -> bool {
    filter.split('/').next() == Some(MESHCORE_ROOT)
}

/// Parse a raw MQTT topic string into its component parts.
///
/// Expected format: `{region}/{iata}/{pubkey}/{subtopic...}`
///
/// # Returns
/// `None` if the topic doesn't have at least 3 segments.
pub fn parse_topic(topic: &str) -> Option<TopicParts> {
    let segments: Vec<&str> = topic.split('/').collect();
    if segments.len() < 3 {
        return None;
    }

    Some(TopicParts {
        region: segments[0].to_string(),
        iata: segments[1].to_string(),
        pubkey: segments[2].to_string(),
        subtopic: segments[3..].iter().map(|s| s.to_string()).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_topic() {
        let parts = parse_topic("us/LAX/abcd1234/telemetry/gps").unwrap();
        assert_eq!(parts.region, "us");
        assert_eq!(parts.iata, "LAX");
        assert_eq!(parts.pubkey, "abcd1234");
        assert_eq!(parts.subtopic, vec!["telemetry", "gps"]);
    }

    #[test]
    fn parse_minimal_topic() {
        let parts = parse_topic("eu/CDG/deadbeef").unwrap();
        assert_eq!(parts.region, "eu");
        assert_eq!(parts.iata, "CDG");
        assert_eq!(parts.pubkey, "deadbeef");
        assert!(parts.subtopic.is_empty());
    }

    #[test]
    fn parse_too_short() {
        assert!(parse_topic("us/LAX").is_none());
        assert!(parse_topic("single").is_none());
        assert!(parse_topic("").is_none());
    }

    #[test]
    fn meshcore_root_filters_allowed() {
        assert!(filter_within_meshcore_root("meshcore/#"));
        assert!(filter_within_meshcore_root("meshcore/SEA/#"));
        assert!(filter_within_meshcore_root("meshcore/SEA/abcd/packets"));
        assert!(filter_within_meshcore_root("meshcore"));
    }

    #[test]
    fn non_meshcore_root_filters_rejected() {
        assert!(!filter_within_meshcore_root("#"));
        assert!(!filter_within_meshcore_root("+/SEA/#"));
        assert!(!filter_within_meshcore_root("$SYS/#"));
        assert!(!filter_within_meshcore_root("us/LAX/abcd"));
        assert!(!filter_within_meshcore_root(""));
    }
}
