use derive_more::with_trait::{Display, From, TryInto};
use std::convert::TryFrom;
use std::fmt;

use juniper::ScalarValue as DeriveScalarValue;
use juniper::parser::Token;
use juniper::{GraphQLScalar, ParseError, ParseScalarResult, Scalar, ScalarToken, ScalarValue};
use serde::{Deserialize, Deserializer, Serialize, de};

#[derive(Clone, Debug, Display, From, PartialEq, ScalarValue, Serialize, TryInto)]
#[serde(untagged)]
pub enum ExtendedScalarValue {
    #[value(to_float, to_int)]
    Int(i32),

    Long(i64),

    #[value(to_float)]
    Float(f64),

    #[value(as_str, to_string)]
    String(String),

    #[value(to_bool)]
    Boolean(bool),
}

impl<'de> Deserialize<'de> for ExtendedScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = ExtendedScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid scalar value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(ExtendedScalarValue::Boolean(b))
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                Ok(ExtendedScalarValue::Int(n))
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
                if let Ok(v) = i32::try_from(n) {
                    Ok(ExtendedScalarValue::Int(v))
                } else {
                    Ok(ExtendedScalarValue::Long(n))
                }
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
                if let Ok(v) = i32::try_from(n) {
                    Ok(ExtendedScalarValue::Int(v))
                } else {
                    self.visit_u64(u64::from(n))
                }
            }

            fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
                if let Ok(v) = i64::try_from(n) {
                    self.visit_i64(v)
                } else {
                    Ok(ExtendedScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(ExtendedScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                Ok(ExtendedScalarValue::String(s.to_owned()))
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(ExtendedScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, GraphQLScalar)]
#[graphql(
    scalar = ExtendedScalarValue,
    name = "Int64",
    with=Self,
)]
pub struct Int64(pub i64);

impl From<Int64> for i64 {
    fn from(value: Int64) -> Self {
        value.0
    }
}

impl From<i64> for Int64 {
    fn from(value: i64) -> Self {
        Self(value)
    }
}

impl Int64 {
    pub(super) fn to_output(&self) -> ExtendedScalarValue {
        ExtendedScalarValue::from(self.0)
    }

    pub(super) fn from_input(v: &Scalar<ExtendedScalarValue>) -> Result<Self, Box<str>> {
        if let Some(raw) = v.downcast_type::<i64>() {
            return Ok(Int64(*raw));
        }
        if let Ok(raw) = v.try_to::<i32>() {
            return Ok(Int64(i64::from(raw)));
        }
        if let Ok(raw) = v.try_to::<f64>() {
            if raw.fract() == 0.0 && raw >= i64::MIN as f64 && raw <= i64::MAX as f64 {
                return Ok(Int64(raw as i64));
            }
            return Err("Expected integer number for Int64".into());
        }
        Err("Expected numeric value for Int64".into())
    }

    pub(super) fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<ExtendedScalarValue> {
        match value {
            ScalarToken::Int(raw) => raw
                .parse::<i64>()
                .map(ExtendedScalarValue::from)
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value))),
            ScalarToken::Float(raw) => raw
                .parse::<f64>()
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value)))
                .and_then(|n| {
                    if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                        Ok(ExtendedScalarValue::from(n as i64))
                    } else {
                        Err(ParseError::unexpected_token(Token::Scalar(value)))
                    }
                }),
            _ => Err(ParseError::unexpected_token(Token::Scalar(value))),
        }
    }
}
