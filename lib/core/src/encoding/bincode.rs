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
