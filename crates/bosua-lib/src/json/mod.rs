//! Thin wrapper over `serde_json` providing a consistent serialization interface.
//!
//! This module decouples the rest of the codebase from direct `serde_json` usage,
//! making it easier to swap or extend the JSON backend in the future.

use serde::{de::DeserializeOwned, Serialize};

/// Serialize a value to a JSON string.
pub fn serialize<T: Serialize>(value: &T) -> crate::errors::Result<String> {
    serde_json::to_string(value).map_err(Into::into)
}

/// Serialize a value to a pretty-printed JSON string.
pub fn serialize_pretty<T: Serialize>(value: &T) -> crate::errors::Result<String> {
    serde_json::to_string_pretty(value).map_err(Into::into)
}

/// Deserialize a JSON string into a value.
pub fn deserialize<T: DeserializeOwned>(json: &str) -> crate::errors::Result<T> {
    serde_json::from_str(json).map_err(Into::into)
}

/// Convert a value to a `serde_json::Value`.
pub fn to_value<T: Serialize>(value: &T) -> crate::errors::Result<serde_json::Value> {
    serde_json::to_value(value).map_err(Into::into)
}

/// Convert a `serde_json::Value` into a typed value.
pub fn from_value<T: DeserializeOwned>(value: serde_json::Value) -> crate::errors::Result<T> {
    serde_json::from_value(value).map_err(Into::into)
}

/// Initialize the JSON adapter. Currently a no-op since serde_json needs no setup,
/// but called during the initialization sequence for parity with the Go version.
pub fn init() {
    // serde_json requires no global initialization.
}
