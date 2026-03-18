/// Fields that are stripped from payloads delivered to limited-role subscribers.
const STRIPPED_FIELDS: &[&str] = &[
    "SNR",
    "RSSI",
    "score",
    "stats",
    "model",
    "firmware_version",
];

/// Filter a message payload for limited-role subscribers.
///
/// Parses the payload as JSON and removes sensitive fields that
/// limited-role subscribers should not receive. Returns `None` if
/// the payload is not valid JSON (pass through as-is in that case).
///
/// # Arguments
/// * `payload` — Raw message payload bytes.
///
/// # Returns
/// * `Some(filtered)` — Filtered JSON payload with stripped fields removed.
/// * `None` — Payload is not JSON; caller should decide whether to forward as-is.
pub fn filter_payload_for_limited(payload: &[u8]) -> Option<Vec<u8>> {
    let mut value: serde_json::Value = serde_json::from_slice(payload).ok()?;

    if let serde_json::Value::Object(ref mut map) = value {
        for field in STRIPPED_FIELDS {
            map.remove(*field);
        }
        Some(serde_json::to_vec(&value).unwrap())
    } else {
        // Not a JSON object — return as-is
        Some(payload.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_sensitive_fields() {
        let input = serde_json::json!({
            "lat": 34.05,
            "lon": -118.25,
            "SNR": 12.5,
            "RSSI": -85,
            "score": 0.95,
            "model": "T-Beam",
            "firmware_version": "1.2.3",
            "stats": {"uptime": 3600},
            "message": "hello"
        });
        let payload = serde_json::to_vec(&input).unwrap();
        let filtered = filter_payload_for_limited(&payload).unwrap();
        let result: serde_json::Value = serde_json::from_slice(&filtered).unwrap();

        assert!(result.get("lat").is_some());
        assert!(result.get("lon").is_some());
        assert!(result.get("message").is_some());
        assert!(result.get("SNR").is_none());
        assert!(result.get("RSSI").is_none());
        assert!(result.get("score").is_none());
        assert!(result.get("model").is_none());
        assert!(result.get("firmware_version").is_none());
        assert!(result.get("stats").is_none());
    }

    #[test]
    fn non_json_returns_none() {
        assert!(filter_payload_for_limited(b"not json").is_none());
    }

    #[test]
    fn json_array_passes_through() {
        let input = serde_json::json!([1, 2, 3]);
        let payload = serde_json::to_vec(&input).unwrap();
        let filtered = filter_payload_for_limited(&payload).unwrap();
        assert_eq!(payload, filtered);
    }
}
