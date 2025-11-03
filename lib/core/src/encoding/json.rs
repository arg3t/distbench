//! JSON serialization format implementation.

use super::{Format, FormatError};
use serde::{Deserialize, Serialize};

/// JSON serialization format.
///
/// This is the default format used by the framework. It produces human-readable
/// JSON output, which is useful for debugging and logging.
#[derive(Clone, Copy, Debug, Default)]
pub struct JsonFormat;

impl Format for JsonFormat {
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, FormatError> {
        serde_json::to_vec(value).map_err(FormatError::JsonSerialization)
    }

    fn deserialize<'de, T: Deserialize<'de>>(&self, bytes: &'de [u8]) -> Result<T, FormatError> {
        serde_json::from_slice(bytes).map_err(FormatError::JsonDeserialization)
    }

    fn name(&self) -> &'static str {
        "json"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestMessage {
        id: u32,
        content: String,
    }

    #[test]
    fn test_json_format() {
        let format = JsonFormat;
        let msg = TestMessage {
            id: 42,
            content: "Hello".to_string(),
        };

        let bytes = format.serialize(&msg).unwrap();
        let decoded: TestMessage = format.deserialize(&bytes).unwrap();

        assert_eq!(msg, decoded);
    }

    #[test]
    fn test_format_name() {
        assert_eq!(JsonFormat.name(), "json");
    }
}
