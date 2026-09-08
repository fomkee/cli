use std::fmt::{self, Debug, Formatter};

use serde::ser::{Error, SerializeMap};
use serde::{Deserialize, Serialize, Serializer, de::DeserializeOwned};
use serde_json::Value;

use crate::error::CliError;

/// A validated API representation together with its lossless machine output.
#[derive(Clone)]
pub struct Response<T> {
    data: T,
    original: Value,
}

impl<T: DeserializeOwned> Response<T> {
    /// Parse the public wire shape without discarding extension fields.
    pub fn parse(original: Value) -> Result<Self, CliError> {
        Self::decode(original).map_err(|_| {
            CliError::MalformedResponse(
                "API response does not match the expected wire shape".into(),
            )
        })
    }

    pub(crate) fn decode(original: Value) -> Result<Self, serde_json::Error> {
        let data = serde_json::from_value(original.clone())?;
        Ok(Self { data, original })
    }
}

impl<T> Response<T> {
    /// Borrow the validated representation, not the unvalidated JSON.
    pub fn data(&self) -> &T {
        &self.data
    }

    pub(crate) fn serialize_with_field<S: Serializer>(
        &self,
        serializer: S,
        name: &str,
        metadata: &impl Serialize,
    ) -> Result<S::Ok, S::Error> {
        let object = self
            .original
            .as_object()
            .ok_or_else(|| S::Error::custom("metadata requires an object response"))?;
        let mut map = serializer.serialize_map(None)?;
        for (key, value) in object {
            if key != name {
                map.serialize_entry(key, value)?;
            }
        }
        map.serialize_entry(name, metadata)?;
        map.end()
    }
}

impl<T> Serialize for Response<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.original.serialize(serializer)
    }
}

impl<T> Debug for Response<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("Response([REDACTED])")
    }
}

/// Content whose intentional wire serialization must never become diagnostics.
#[derive(Clone, Deserialize, Serialize)]
#[serde(transparent)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wrap content at its input boundary.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub(crate) fn expose(&self) -> &T {
        &self.0
    }
}

impl<T> Debug for Secret<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_str("[REDACTED]")
    }
}
