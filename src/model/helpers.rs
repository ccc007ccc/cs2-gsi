//! Helpers for deserializing the quirks of CS2's GSI payload.
//!
//! The CS2 client sends many numeric fields as JSON strings (e.g.
//! `"health": "100"` instead of `"health": 100`). The helpers in this module
//! accept either form and normalize them on the way in.

use serde::{Deserialize, Deserializer};
use std::fmt::Display;
use std::str::FromStr;

/// Deserialize a value that may arrive as either a JSON number or a JSON
/// string containing a number. Used for fields like `health`, `armor`, etc.
pub(crate) fn de_num_or_str<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AnyNum<T> {
        Str(String),
        Val(T),
    }

    match AnyNum::<T>::deserialize(deserializer)? {
        AnyNum::Val(v) => Ok(v),
        AnyNum::Str(s) => s.trim().parse::<T>().map_err(serde::de::Error::custom),
    }
}

/// Same as [`de_num_or_str`] but yields `Option<T>`. Missing fields, JSON
/// `null` values and empty strings all collapse to `None`.
pub(crate) fn de_opt_num_or_str<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr + Deserialize<'de>,
    <T as FromStr>::Err: Display,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum AnyNum<T> {
        Null,
        Str(String),
        Val(T),
    }

    let raw = Option::<AnyNum<T>>::deserialize(deserializer)?;
    match raw {
        None | Some(AnyNum::Null) => Ok(None),
        Some(AnyNum::Val(v)) => Ok(Some(v)),
        Some(AnyNum::Str(s)) => {
            let s = s.trim();
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse::<T>().map(Some).map_err(serde::de::Error::custom)
            }
        }
    }
}
