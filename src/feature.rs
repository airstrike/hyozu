//! Feature identifier — a newtype over `String` shared by choropleth
//! polygons, bubble-map points, and the `Target::Feature` variant emitted
//! on click.
//!
//! Exists as a newtype so that the id flowing through `GeoFeature.id` →
//! `ChoroplethEntry.id` / `MapPoint.id` → `Target::Feature { id }` is a
//! distinct type, not another bare `String`. Call sites stay ergonomic
//! via `impl Into<Id>` on builders, backed by `From<String>` and
//! `From<&str>` impls.

/// Stable feature identifier (country code, state code, polygon id, etc.).
/// Constructed from a string via `Id::new` or `.into()`.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id(String);

impl Id {
    /// Construct an `Id` from anything that converts into a `String`.
    pub fn new(s: impl Into<String>) -> Self {
        Self(s.into())
    }

    /// Borrow the id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the id and return the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl From<String> for Id {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&String> for Id {
    fn from(value: &String) -> Self {
        Self(value.clone())
    }
}

impl From<&str> for Id {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl AsRef<str> for Id {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for Id {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl PartialEq<str> for Id {
    fn eq(&self, other: &str) -> bool {
        self.0 == other
    }
}

impl PartialEq<&str> for Id {
    fn eq(&self, other: &&str) -> bool {
        self.0 == *other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn id_roundtrips_from_str_and_string() {
        let from_str: Id = "US".into();
        let from_string: Id = String::from("US").into();
        assert_eq!(from_str, from_string);
        assert_eq!(from_str.as_str(), "US");
    }

    #[test]
    fn id_compares_to_str_literal() {
        let id: Id = "CA".into();
        assert!(id == "CA");
        assert!(id != "NY");
    }

    #[test]
    fn id_displays_as_inner_string() {
        let id: Id = "World".into();
        assert_eq!(format!("{id}"), "World");
    }

    #[test]
    fn id_into_inner_returns_owned_string() {
        let id: Id = "Japan".into();
        assert_eq!(id.into_inner(), String::from("Japan"));
    }
}
