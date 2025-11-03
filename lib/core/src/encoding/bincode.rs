//! Bincode serialization format implementation.

use super::{Format, FormatError};
use serde::{Deserialize, Serialize};

/// Bincode serialization format.
///
/// This format produces compact binary output, which is more efficient than JSON
/// in terms of both size and performance. However, it is not human-readable.
#[derive(Clone, Copy, Debug, Default)]
pub struct BincodeFormat;

impl Format for BincodeFormat {
    fn serialize<T: Serialize>(&self, value: &T) -> Result<Vec<u8>, FormatError> {
        bincode::serialize(value).map_err(FormatError::BincodeSerialization)
    }

    fn deserialize<'de, T: Deserialize<'de>>(&self, bytes: &'de [u8]) -> Result<T, FormatError> {
        bincode::deserialize(bytes).map_err(FormatError::BincodeDeserialization)
    }

    fn name(&self) -> &'static str {
        "bincode"
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
    fn test_bincode_format() {
        let format = BincodeFormat;
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
        assert_eq!(BincodeFormat.name(), "bincode");
    }
}
