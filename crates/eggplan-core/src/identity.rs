use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};
use thiserror::Error;

/// Opaque identity of a canonical verification specification.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(try_from = "String", into = "String")]
pub struct VerificationDigest(String);

impl VerificationDigest {
    pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
        let value = value.into();
        let valid = value.strip_prefix("sha256:").is_some_and(|hex| {
            hex.len() == 64
                && hex
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        });
        if valid {
            Ok(Self(value))
        } else {
            Err(IdError::Invalid("verification digest"))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for VerificationDigest {
    type Error = IdError;
    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}
impl From<VerificationDigest> for String {
    fn from(value: VerificationDigest) -> Self {
        value.0
    }
}

#[cfg(test)]
mod verification_digest_tests {
    use super::*;

    #[test]
    fn verification_digest_requires_sha256_lowercase_hex() {
        assert!(VerificationDigest::new(format!("sha256:{}", "a".repeat(64))).is_ok());
        assert!(VerificationDigest::new(format!("sha256:{}", "A".repeat(64))).is_err());
        assert!(VerificationDigest::new("sha256:abc").is_err());
        assert!(
            serde_json::from_str::<VerificationDigest>(&format!("\"sha256:{}\"", "G".repeat(64)))
                .is_err()
        );
    }
}
use uuid::Uuid;

use crate::bounds::ID_CHARS;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum IdError {
    #[error("identifier is empty or too long")]
    Length,
    #[error("identifier must start with {expected}")]
    Prefix { expected: &'static str },
    #[error("identifier contains an invalid character")]
    Character,
    #[error("invalid {0}")]
    Invalid(&'static str),
}

pub trait TypedId: Sized + Clone + Eq + Ord + fmt::Display {
    const PREFIX: &'static str;
    fn as_str(&self) -> &str;
    fn parse(value: impl Into<String>) -> Result<Self, IdError>;
    fn generate() -> Self;
}

macro_rules! define_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, IdError> {
                Self::parse(value)
            }
            pub fn generate() -> Self {
                <Self as TypedId>::generate()
            }
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl TypedId for $name {
            const PREFIX: &'static str = $prefix;
            fn as_str(&self) -> &str {
                &self.0
            }
            fn parse(value: impl Into<String>) -> Result<Self, IdError> {
                let value = value.into();
                if value.is_empty() || value.chars().count() > ID_CHARS {
                    return Err(IdError::Length);
                }
                if !value.starts_with($prefix) {
                    return Err(IdError::Prefix { expected: $prefix });
                }
                if !value[$prefix.len()..]
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    return Err(IdError::Character);
                }
                Ok(Self(value))
            }
            fn generate() -> Self {
                Self(format!("{}{}", $prefix, Uuid::new_v4().simple()))
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
        impl FromStr for $name {
            type Err = IdError;
            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Self::parse(value)
            }
        }
        impl TryFrom<String> for $name {
            type Error = IdError;
            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(value)
            }
        }
        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

define_id!(PlanId, "ep_");
define_id!(PlanItemId, "epi_");
define_id!(CriterionId, "epc_");
define_id!(EvidenceProviderId, "epp_");
define_id!(EvidenceObservationId, "epe_");
define_id!(ClosureId, "epcl_");
define_id!(EvidenceSupersessionId, "eps_");

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_roundtrip_generate_and_reject_bad_prefixes() {
        let id = PlanId::new("ep_example-1").unwrap();
        assert_eq!(id.to_string(), "ep_example-1");
        assert_eq!(id.as_str().parse::<PlanId>().unwrap(), id);
        assert!(PlanId::new("epi_wrong").is_err());
        assert!(PlanItemId::new("epi_has space").is_err());
        assert!(CriterionId::new(format!("epc_{}", "x".repeat(crate::bounds::ID_CHARS))).is_err());
        assert!(EvidenceProviderId::generate().as_str().starts_with("epp_"));
    }
}
