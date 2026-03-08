use std::convert::TryFrom;
use std::fmt;

use juniper::ScalarValue as DeriveScalarValue;
use juniper::parser::Token;
use juniper::{GraphQLScalar, ParseError, ParseScalarResult, Scalar, ScalarToken};
use serde::{Deserialize, Deserializer, Serialize, de};

#[derive(Clone, Debug, PartialEq, Serialize, DeriveScalarValue)]
#[serde(untagged)]
pub enum MyScalarValue {
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

impl fmt::Display for MyScalarValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Int(v) => write!(f, "{v}"),
            Self::Long(v) => write!(f, "{v}"),
            Self::Float(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "{v}"),
            Self::Boolean(v) => write!(f, "{v}"),
        }
    }
}

impl From<i32> for MyScalarValue {
    fn from(value: i32) -> Self {
        Self::Int(value)
    }
}

impl From<i64> for MyScalarValue {
    fn from(value: i64) -> Self {
        if let Ok(v) = i32::try_from(value) { Self::Int(v) } else { Self::Long(value) }
    }
}

impl From<f64> for MyScalarValue {
    fn from(value: f64) -> Self {
        Self::Float(value)
    }
}

impl From<bool> for MyScalarValue {
    fn from(value: bool) -> Self {
        Self::Boolean(value)
    }
}

impl From<String> for MyScalarValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl TryFrom<MyScalarValue> for String {
    type Error = &'static str;

    fn try_from(value: MyScalarValue) -> Result<Self, Self::Error> {
        match value {
            MyScalarValue::String(v) => Ok(v),
            _ => Err("Not a string scalar"),
        }
    }
}

impl<'de> Deserialize<'de> for MyScalarValue {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = MyScalarValue;

            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("a valid scalar value")
            }

            fn visit_bool<E: de::Error>(self, b: bool) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Boolean(b))
            }

            fn visit_i32<E: de::Error>(self, n: i32) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Int(n))
            }

            fn visit_i64<E: de::Error>(self, n: i64) -> Result<Self::Value, E> {
                if let Ok(v) = i32::try_from(n) {
                    Ok(MyScalarValue::Int(v))
                } else {
                    Ok(MyScalarValue::Long(n))
                }
            }

            fn visit_u32<E: de::Error>(self, n: u32) -> Result<Self::Value, E> {
                if let Ok(v) = i32::try_from(n) {
                    Ok(MyScalarValue::Int(v))
                } else {
                    self.visit_u64(u64::from(n))
                }
            }

            fn visit_u64<E: de::Error>(self, n: u64) -> Result<Self::Value, E> {
                if let Ok(v) = i64::try_from(n) {
                    self.visit_i64(v)
                } else {
                    Ok(MyScalarValue::Float(n as f64))
                }
            }

            fn visit_f64<E: de::Error>(self, f: f64) -> Result<Self::Value, E> {
                Ok(MyScalarValue::Float(f))
            }

            fn visit_str<E: de::Error>(self, s: &str) -> Result<Self::Value, E> {
                Ok(MyScalarValue::String(s.to_owned()))
            }

            fn visit_string<E: de::Error>(self, s: String) -> Result<Self::Value, E> {
                Ok(MyScalarValue::String(s))
            }
        }

        de.deserialize_any(Visitor)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, GraphQLScalar)]
#[graphql(
    scalar = MyScalarValue,
    name = "Int64",
    to_output_with = int64_scalar::to_output,
    from_input_with = int64_scalar::from_input,
    parse_token_with = int64_scalar::parse_token
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

mod int64_scalar {
    use super::*;

    pub(super) fn to_output(v: &Int64) -> MyScalarValue {
        MyScalarValue::from(v.0)
    }

    pub(super) fn from_input(v: &Scalar<MyScalarValue>) -> Result<Int64, Box<str>> {
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

    pub(super) fn parse_token(value: ScalarToken<'_>) -> ParseScalarResult<MyScalarValue> {
        match value {
            ScalarToken::Int(raw) => raw
                .parse::<i64>()
                .map(MyScalarValue::from)
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value))),
            ScalarToken::Float(raw) => raw
                .parse::<f64>()
                .map_err(|_| ParseError::unexpected_token(Token::Scalar(value)))
                .and_then(|n| {
                    if n.fract() == 0.0 && n >= i64::MIN as f64 && n <= i64::MAX as f64 {
                        Ok(MyScalarValue::from(n as i64))
                    } else {
                        Err(ParseError::unexpected_token(Token::Scalar(value)))
                    }
                }),
            _ => Err(ParseError::unexpected_token(Token::Scalar(value))),
        }
    }
}
