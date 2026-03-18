use crate::types::TopicParts;

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
}
