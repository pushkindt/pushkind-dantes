//! Strongly-typed value objects used by domain entities.
//!
//! Domain structs should carry these wrappers instead of raw primitives so that
//! identifiers, text values and numeric constraints are enforced at the
//! boundary.

use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use thiserror::Error;
use validator::ValidateUrl;

/// Errors produced when attempting to construct constrained domain types.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TypeConstraintError {
    /// An identifier was zero or negative.
    #[error("{0} must be greater than zero")]
    NonPositiveId(&'static str),
    /// A numeric value required to be positive was zero/negative or invalid.
    #[error("{0} must be greater than zero")]
    NonPositiveNumber(&'static str),
    /// A numeric value required to be non-negative was negative.
    #[error("{0} must be zero or greater")]
    NegativeNumber(&'static str),
    /// A string was empty or whitespace-only after trimming.
    #[error("{0} cannot be empty")]
    EmptyString(&'static str),
    /// URL validation failed.
    #[error("{0} must be a valid URL")]
    InvalidUrl(&'static str),
    /// Similarity distance must be in [0.0, 1.0].
    #[error("similarity distance must be between 0.0 and 1.0")]
    InvalidSimilarityDistance,
    /// Catch-all for custom validation failures.
    #[error("invalid value: {0}")]
    InvalidValue(String),
}

fn trim_and_require_non_empty<S: Into<String>>(
    value: S,
    field: &'static str,
) -> Result<String, TypeConstraintError> {
    let trimmed = value.into().trim().to_string();
    if trimmed.is_empty() {
        Err(TypeConstraintError::EmptyString(field))
    } else {
        Ok(trimmed)
    }
}

/// Wrapper for non-empty, trimmed strings.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(transparent)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    /// Trims whitespace and rejects empty inputs.
    pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
        Self::new_for_field(value, "value")
    }

    /// Same as [`Self::new`] but with field-specific error context.
    pub fn new_for_field<S: Into<String>>(
        value: S,
        field: &'static str,
    ) -> Result<Self, TypeConstraintError> {
        trim_and_require_non_empty(value, field).map(Self)
    }

    /// Borrow the inner string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the wrapper returning the owned string.
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for NonEmptyString {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.as_str()
    }
}

impl AsRef<str> for NonEmptyString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl TryFrom<String> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<NonEmptyString> for String {
    fn from(value: NonEmptyString) -> Self {
        value.0
    }
}

/// Macro to generate lightweight newtypes for positive identifiers.
macro_rules! id_newtype {
    ($name:ident, $doc:expr, $field:expr) => {
        #[doc = $doc]
        #[derive(
            Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord,
        )]
        #[serde(transparent)]
        pub struct $name(i32);

        impl $name {
            /// Creates a new identifier ensuring it is greater than zero.
            pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
                if value > 0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NonPositiveId($field))
                }
            }

            /// Returns the raw `i32` backing this identifier.
            pub const fn get(self) -> i32 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<i32> for $name {
            fn eq(&self, other: &i32) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for i32 {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }
    };
}

macro_rules! non_empty_string_newtype {
    ($name:ident, $doc:expr, $field:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Constructs a trimmed, non-empty value.
            pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
                let inner = NonEmptyString::new_for_field(value, $field)?;
                Ok(Self(inner.into_inner()))
            }

            /// Borrow the value as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume the wrapper and return the owned string.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl TryFrom<String> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.as_str()
            }
        }
    };
}

macro_rules! url_string_newtype {
    ($name:ident, $doc:expr, $field:expr) => {
        #[doc = $doc]
        #[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            /// Constructs a trimmed URL and validates its format.
            pub fn new<S: Into<String>>(value: S) -> Result<Self, TypeConstraintError> {
                let trimmed = trim_and_require_non_empty(value, $field)?;
                if !trimmed.as_str().validate_url() {
                    return Err(TypeConstraintError::InvalidUrl($field));
                }
                Ok(Self(trimmed))
            }

            /// Borrow the URL as a string slice.
            pub fn as_str(&self) -> &str {
                &self.0
            }

            /// Consume the wrapper and return the owned URL.
            pub fn into_inner(self) -> String {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl std::ops::Deref for $name {
            type Target = str;

            fn deref(&self) -> &Self::Target {
                self.as_str()
            }
        }

        impl AsRef<str> for $name {
            fn as_ref(&self) -> &str {
                self.as_str()
            }
        }

        impl TryFrom<String> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl TryFrom<&str> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: &str) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<&str> for $name {
            fn eq(&self, other: &&str) -> bool {
                self.as_str() == *other
            }
        }

        impl PartialEq<$name> for &str {
            fn eq(&self, other: &$name) -> bool {
                *self == other.as_str()
            }
        }
    };
}

macro_rules! positive_f64_newtype {
    ($name:ident, $doc:expr, $field:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
        #[serde(transparent)]
        pub struct $name(f64);

        impl $name {
            /// Constructs a strictly positive, finite numeric value.
            pub fn new(value: f64) -> Result<Self, TypeConstraintError> {
                if value.is_finite() && value > 0.0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NonPositiveNumber($field))
                }
            }

            /// Returns the raw `f64` value.
            pub const fn get(self) -> f64 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<f64> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: f64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for f64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<f64> for $name {
            fn eq(&self, other: &f64) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for f64 {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }
    };
}

macro_rules! non_negative_f64_newtype {
    ($name:ident, $doc:expr, $field:expr) => {
        #[doc = $doc]
        #[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
        #[serde(transparent)]
        pub struct $name(f64);

        impl $name {
            /// Constructs a finite numeric value that is zero or greater.
            pub fn new(value: f64) -> Result<Self, TypeConstraintError> {
                if value.is_finite() && value >= 0.0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NegativeNumber($field))
                }
            }

            /// Returns the raw `f64` value.
            pub const fn get(self) -> f64 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<f64> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: f64) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for f64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<f64> for $name {
            fn eq(&self, other: &f64) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for f64 {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }
    };
}

macro_rules! non_negative_i32_newtype {
    ($name:ident, $doc:expr, $field:expr) => {
        #[doc = $doc]
        #[derive(
            Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord,
        )]
        #[serde(transparent)]
        pub struct $name(i32);

        impl $name {
            /// Constructs a value that must be zero or greater.
            pub fn new(value: i32) -> Result<Self, TypeConstraintError> {
                if value >= 0 {
                    Ok(Self(value))
                } else {
                    Err(TypeConstraintError::NegativeNumber($field))
                }
            }

            /// Returns the raw `i32` value.
            pub const fn get(self) -> i32 {
                self.0
            }
        }

        impl Display for $name {
            fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl TryFrom<i32> for $name {
            type Error = TypeConstraintError;

            fn try_from(value: i32) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for i32 {
            fn from(value: $name) -> Self {
                value.0
            }
        }

        impl PartialEq<i32> for $name {
            fn eq(&self, other: &i32) -> bool {
                self.0 == *other
            }
        }

        impl PartialEq<$name> for i32 {
            fn eq(&self, other: &$name) -> bool {
                *self == other.0
            }
        }
    };
}

id_newtype!(HubId, "Unique identifier for a hub.", "hub_id");
id_newtype!(CrawlerId, "Unique identifier for a crawler.", "crawler_id");
id_newtype!(ProductId, "Unique identifier for a product.", "product_id");
id_newtype!(
    CategoryId,
    "Unique identifier for a category.",
    "category_id"
);
id_newtype!(
    BenchmarkId,
    "Unique identifier for a benchmark.",
    "benchmark_id"
);

non_empty_string_newtype!(
    CrawlerName,
    "Crawler display name enforcing non-empty values.",
    "crawler name"
);
non_empty_string_newtype!(
    CrawlerSelectorValue,
    "Crawler selector token/value enforcing non-empty values.",
    "crawler selector"
);
non_empty_string_newtype!(
    BenchmarkName,
    "Benchmark name enforcing non-empty values.",
    "benchmark name"
);
non_empty_string_newtype!(
    BenchmarkSku,
    "Benchmark SKU enforcing non-empty values.",
    "benchmark sku"
);
non_empty_string_newtype!(
    CategoryName,
    "Category name enforcing non-empty values.",
    "category"
);
non_empty_string_newtype!(
    ProductName,
    "Product name enforcing non-empty values.",
    "product name"
);
non_empty_string_newtype!(
    ProductSku,
    "Product SKU enforcing non-empty values.",
    "product sku"
);
non_empty_string_newtype!(
    ProductUnits,
    "Product units enforcing non-empty values.",
    "units"
);
non_empty_string_newtype!(
    ProductDescription,
    "Product description enforcing non-empty values.",
    "description"
);

url_string_newtype!(CrawlerUrl, "Crawler URL.", "crawler url");
url_string_newtype!(ProductUrl, "Product URL.", "product url");
url_string_newtype!(ImageUrl, "Product image URL.", "image url");

non_negative_f64_newtype!(
    ProductPrice,
    "Non-negative price value in standard currency units.",
    "price"
);
positive_f64_newtype!(ProductAmount, "Positive product amount/quantity.", "amount");

non_negative_i32_newtype!(
    ProductCount,
    "Number of products associated with an entity.",
    "product count"
);

/// Source of a product's canonical category assignment.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum CategoryAssignmentSource {
    Automatic,
    Manual,
}

impl CategoryAssignmentSource {
    /// String representation used in persistence.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Automatic => "automatic",
            Self::Manual => "manual",
        }
    }
}

impl Display for CategoryAssignmentSource {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl TryFrom<&str> for CategoryAssignmentSource {
    type Error = TypeConstraintError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value.trim() {
            "automatic" => Ok(Self::Automatic),
            "manual" => Ok(Self::Manual),
            other => Err(TypeConstraintError::InvalidValue(format!(
                "category assignment source: {other}"
            ))),
        }
    }
}

impl TryFrom<String> for CategoryAssignmentSource {
    type Error = TypeConstraintError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl From<CategoryAssignmentSource> for String {
    fn from(value: CategoryAssignmentSource) -> Self {
        value.as_str().to_string()
    }
}

/// Similarity distance between benchmark and product in the inclusive range [0.0, 1.0].
#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, PartialOrd)]
#[serde(transparent)]
pub struct SimilarityDistance(f32);

impl SimilarityDistance {
    /// Constructs a validated similarity distance.
    pub fn new(value: f32) -> Result<Self, TypeConstraintError> {
        if value.is_finite() && (0.0..=1.0).contains(&value) {
            Ok(Self(value))
        } else {
            Err(TypeConstraintError::InvalidSimilarityDistance)
        }
    }

    /// Returns the raw `f32` value.
    pub const fn get(self) -> f32 {
        self.0
    }
}

impl Display for SimilarityDistance {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TryFrom<f32> for SimilarityDistance {
    type Error = TypeConstraintError;

    fn try_from(value: f32) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<SimilarityDistance> for f32 {
    fn from(value: SimilarityDistance) -> Self {
        value.0
    }
}

impl PartialEq<f32> for SimilarityDistance {
    fn eq(&self, other: &f32) -> bool {
        self.0 == *other
    }
}

impl PartialEq<SimilarityDistance> for f32 {
    fn eq(&self, other: &SimilarityDistance) -> bool {
        *self == other.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trims_non_empty_strings() {
        let value = NonEmptyString::new("  product  ").unwrap();
        assert_eq!(value.as_str(), "product");
    }

    #[test]
    fn rejects_non_positive_ids() {
        let err = ProductId::new(0).unwrap_err();
        assert_eq!(err, TypeConstraintError::NonPositiveId("product_id"));
    }

    #[test]
    fn validates_urls() {
        assert!(ProductUrl::new("https://example.com/p/123").is_ok());
        let err = ProductUrl::new("not-a-url").unwrap_err();
        assert_eq!(err, TypeConstraintError::InvalidUrl("product url"));
    }

    #[test]
    fn validates_similarity_distance_range() {
        assert!(SimilarityDistance::new(0.0).is_ok());
        assert!(SimilarityDistance::new(1.0).is_ok());
        assert_eq!(
            SimilarityDistance::new(1.1).unwrap_err(),
            TypeConstraintError::InvalidSimilarityDistance
        );
    }

    #[test]
    fn product_price_allows_zero() {
        assert_eq!(ProductPrice::new(0.0).unwrap().get(), 0.0);
    }

    #[test]
    fn product_price_rejects_negative_numbers() {
        assert_eq!(
            ProductPrice::new(-0.01).unwrap_err(),
            TypeConstraintError::NegativeNumber("price")
        );
    }
}
