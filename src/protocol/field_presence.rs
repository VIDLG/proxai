use derive_more::From;
use serde::de::Error as _;
use serde::ser::Error as _;
use serde::{Deserialize, Deserializer, Serialize};

/// A wire field value that is present but may be JSON `null`.
///
/// This carrier represents only the value/null distinction. Field presence is
/// expressed by [`RequiredNullable`] or [`OptionalNullable`] at the containing
/// struct field. Callers must explicitly use [`Nullable::as_non_null`] or
/// [`Nullable::into_non_null`] when collapsing JSON `null` into Rust `None`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, From, Serialize)]
#[serde(transparent)]
pub struct Nullable<T>(Option<T>);

impl<'de, T> Deserialize<'de> for Nullable<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Protocol carriers are JSON wire types. Buffering one field as `Value`
        // lets serde distinguish a missing field from explicit `null`, while
        // deserializing non-null values through `T` preserves useful inner
        // errors such as unknown enum variants.
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_null() {
            return Ok(Self::null());
        }
        T::deserialize(value)
            .map(|value| Self(Some(value)))
            .map_err(D::Error::custom)
    }
}

impl<T> Nullable<T> {
    pub const fn null() -> Self {
        Self(None)
    }

    /// Returns `true` when the wire value is JSON `null`.
    pub fn is_null(&self) -> bool {
        self.0.is_none()
    }

    /// Returns `true` when the wire value contains a non-null value.
    pub fn is_non_null(&self) -> bool {
        self.0.is_some()
    }

    /// Consumes the carrier, collapsing JSON `null` into `None`.
    pub fn into_non_null(self) -> Option<T> {
        self.0
    }

    /// Borrows the contained value, collapsing JSON `null` into `None`.
    pub fn as_non_null(&self) -> Option<&T> {
        self.0.as_ref()
    }

    /// Maps a non-null value while preserving JSON `null`.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> Nullable<U> {
        Nullable(self.0.map(map))
    }
}

impl<T> Default for Nullable<T> {
    fn default() -> Self {
        Self::null()
    }
}

impl<T> From<T> for Nullable<T> {
    fn from(value: T) -> Self {
        Self(Some(value))
    }
}

/// A required field whose present value may be JSON `null`.
///
/// The carrier's `Deserialize` implementation rejects a missing field while
/// accepting explicit JSON `null`, so no field-level serde attribute is needed.
pub type RequiredNullable<T> = Nullable<T>;

/// An optional field that distinguishes missing, JSON `null`, and a value.
///
/// Its `Deserialize` implementation handles only fields present on the wire;
/// `#[serde(default)]` selects [`OptionalNullable::Missing`] for an omitted
/// field. Fields must skip [`OptionalNullable::Missing`] when serializing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionalNullable<T> {
    #[default]
    Missing,
    Null,
    Value(T),
}

impl<T> OptionalNullable<T> {
    pub const fn missing() -> Self {
        Self::Missing
    }

    pub const fn null() -> Self {
        Self::Null
    }

    /// Returns `true` when the field was omitted from the wire payload.
    pub fn is_missing(&self) -> bool {
        matches!(self, Self::Missing)
    }

    /// Returns `true` when the field was present with JSON `null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns `true` when the field was present with a non-null value.
    pub fn is_non_null(&self) -> bool {
        matches!(self, Self::Value(_))
    }

    /// Consumes the carrier, collapsing missing and JSON `null` into `None`.
    pub fn into_non_null(self) -> Option<T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Missing | Self::Null => None,
        }
    }

    /// Borrows the contained value, collapsing missing and JSON `null` into `None`.
    pub fn as_non_null(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Missing | Self::Null => None,
        }
    }

    /// Consumes the carrier, collapsing missing and JSON `null` into a default value.
    pub fn unwrap_or_default(self) -> T
    where
        T: Default,
    {
        self.into_non_null().unwrap_or_default()
    }

    /// Maps a non-null value while preserving missing and JSON `null`.
    pub fn map<U>(self, map: impl FnOnce(T) -> U) -> OptionalNullable<U> {
        match self {
            Self::Missing => OptionalNullable::Missing,
            Self::Null => OptionalNullable::Null,
            Self::Value(value) => OptionalNullable::Value(map(value)),
        }
    }
}

impl<T> From<T> for OptionalNullable<T> {
    fn from(value: T) -> Self {
        Self::Value(value)
    }
}

impl<T> From<Option<T>> for OptionalNullable<T> {
    fn from(value: Option<T>) -> Self {
        match value {
            None => Self::Missing,
            Some(value) => Self::Value(value),
        }
    }
}

impl<T> Serialize for OptionalNullable<T>
where
    T: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Missing => Err(S::Error::custom(
                "cannot serialize a missing optional-nullable field without skip_serializing_if",
            )),
            Self::Null => serializer.serialize_none(),
            Self::Value(value) => value.serialize(serializer),
        }
    }
}

impl<'de, T> Deserialize<'de> for OptionalNullable<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Nullable::deserialize(deserializer).map(|value| match value.into_non_null() {
            Some(value) => Self::Value(value),
            None => Self::Null,
        })
    }
}

/// Deserialize a field known to be present without allowing the outer `Option`
/// to collapse JSON `null` into the same state as a missing field.
pub fn deserialize_present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[cfg(test)]
#[path = "field_presence_tests.rs"]
mod tests;
