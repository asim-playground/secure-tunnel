// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use serde::{Deserialize, Serialize};

use crate::error::{SdkError, SdkResult};

/// Parsed and validated bootstrap descriptor used to start a tunnel session.
#[derive(Debug, Clone)]
pub struct BootstrapDescriptor {
    descriptor: secure_tunnel_core::ServiceDescriptor,
    normalized_json: String,
}

impl BootstrapDescriptor {
    /// Parses, validates, and normalizes a service descriptor JSON document.
    ///
    /// # Errors
    ///
    /// Returns [`SdkErrorKind::InvalidDescriptor`](crate::SdkErrorKind::InvalidDescriptor)
    /// when the JSON cannot decode into a v1 descriptor or fails validation.
    /// Returns [`SdkErrorKind::Internal`](crate::SdkErrorKind::Internal) if the
    /// validated descriptor cannot be re-encoded.
    pub fn from_json(descriptor_json: &str) -> SdkResult<Self> {
        let descriptor =
            serde_json::from_str::<secure_tunnel_core::ServiceDescriptor>(descriptor_json)
                .map_err(|error| SdkError::invalid_descriptor(error.to_string()))?;
        descriptor
            .validate()
            .map_err(|error| SdkError::from_core(&error))?;
        let normalized_json = serde_json::to_string(&descriptor)
            .map_err(|error| SdkError::internal(error.to_string()))?;

        Ok(Self {
            descriptor,
            normalized_json,
        })
    }

    /// Returns a validated example descriptor JSON document.
    ///
    /// # Errors
    ///
    /// Returns [`SdkErrorKind::Internal`](crate::SdkErrorKind::Internal) if the
    /// built-in example descriptor cannot be encoded.
    pub fn example_json() -> SdkResult<String> {
        serde_json::to_string(&secure_tunnel_core::example_service_descriptor())
            .map_err(|error| SdkError::internal(error.to_string()))
    }

    /// Returns the normalized descriptor JSON.
    #[must_use]
    pub fn normalized_json(&self) -> String {
        self.normalized_json.clone()
    }

    /// Returns the descriptor environment identifier.
    #[must_use]
    pub fn environment_id(&self) -> String {
        self.descriptor.environment_id.clone()
    }

    /// Returns the descriptor service identifier.
    #[must_use]
    pub fn service_id(&self) -> String {
        self.descriptor.service_id.clone()
    }

    pub(super) const fn core_descriptor(&self) -> &secure_tunnel_core::ServiceDescriptor {
        &self.descriptor
    }
}

/// Transport selection policy supplied by SDK callers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportPolicyConfig {
    /// Seconds to wait before retrying `QUIC` after a fallback-eligible failure.
    pub quic_reprobe_delay_seconds: u64,
}

impl Default for TransportPolicyConfig {
    fn default() -> Self {
        Self {
            quic_reprobe_delay_seconds: 300,
        }
    }
}
