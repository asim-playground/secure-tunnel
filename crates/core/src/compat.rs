// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use thiserror::Error;

/// Temporary scaffold parser error retained for binding compatibility.
#[derive(Error, Debug, Clone)]
pub enum ParseError {
    /// A generic parsing error with a descriptive message.
    #[error("Parse error: {0}")]
    Generic(String),
}

/// Temporary scaffold parser retained while language bindings are still
/// generated against the bootstrap repository template.
///
/// # Errors
///
/// Returns a `ParseError` when the input is empty, malformed, or overflows.
pub fn parse(input: &str) -> Result<String, ParseError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(ParseError::Generic("input is empty".to_owned()));
    }

    let mut total = 0_i64;
    for part in trimmed.split('+') {
        let number = part.trim();
        if number.is_empty() {
            return Err(ParseError::Generic(
                "expected a number between '+' operators".to_owned(),
            ));
        }

        let value = number
            .parse::<i64>()
            .map_err(|error| ParseError::Generic(format!("invalid integer `{number}`: {error}")))?;
        total = total.checked_add(value).ok_or_else(|| {
            ParseError::Generic(format!(
                "integer overflow while summing expression near `{number}`"
            ))
        })?;
    }

    Ok(total.to_string())
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn parse_sums_basic_addition() {
        assert_eq!(parse("1+2+3").unwrap(), "6");
        assert_eq!(parse("10+20+30").unwrap(), "60");
    }

    #[test]
    fn parse_rejects_missing_operands() {
        let error = parse("1+").unwrap_err();
        assert!(error.to_string().contains("expected a number"));
    }

    #[test]
    fn parse_rejects_overflow() {
        let error = parse("9223372036854775807+1").unwrap_err();
        assert!(error.to_string().contains("integer overflow"));
    }
}
