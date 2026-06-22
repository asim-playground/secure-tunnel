// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use crate::error::{FfiResult, invalid_config};
use crate::types::{
    AccountAuthMode, AccountAuthReport, ClientConfig, ConnectReport, DescriptorTrustAnchor,
    TransportAttemptReport, TransportCacheSnapshot,
};
use crate::types_more::{
    account_freshness, attempt_outcome, cache_state, candidate_source, carrier,
    close_classification, fallback_reason, session_state,
};

pub fn sdk_config(config: ClientConfig) -> FfiResult<secure_tunnel_sdk::ClientConfig> {
    let mut pinned = Vec::with_capacity(config.pinned_service_static_public_keys.len());
    for key in config.pinned_service_static_public_keys {
        pinned.push(
            key.try_into().map_err(|_| {
                invalid_config("pinned service static public keys must be 32 bytes")
            })?,
        );
    }
    let outer_roots = if config.outer_root_certificates_der.is_empty() {
        None
    } else {
        Some(config.outer_root_certificates_der)
    };
    Ok(secure_tunnel_sdk::ClientConfig {
        transport_policy: secure_tunnel_sdk::TransportPolicyConfig {
            quic_reprobe_delay_seconds: config.quic_reprobe_delay_seconds,
            connect_timeout_ms: config.connect_timeout_ms,
            quic_connect_timeout_ms: config.quic_connect_timeout_ms,
            wss_connect_timeout_ms: config.wss_connect_timeout_ms,
            secure_ready_timeout_ms: config.secure_ready_timeout_ms,
            record_read_timeout_ms: config.record_read_timeout_ms,
            record_write_timeout_ms: config.record_write_timeout_ms,
        },
        outer_root_certificates_der: outer_roots,
        descriptor_trust_anchors: config
            .descriptor_trust_anchors
            .into_iter()
            .map(sdk_trust_anchor)
            .collect(),
        pinned_service_static_public_keys: pinned,
    })
}

fn sdk_trust_anchor(anchor: DescriptorTrustAnchor) -> secure_tunnel_core::TrustAnchor {
    secure_tunnel_core::TrustAnchor {
        key_id: anchor.key_id,
        algorithm: anchor.algorithm,
        public_key: anchor.public_key,
    }
}

pub fn sdk_cache(cache: &TransportCacheSnapshot) -> secure_tunnel_sdk::TransportCacheSnapshot {
    secure_tunnel_sdk::TransportCacheSnapshot {
        last_successful_carrier: cache.last_successful_carrier.map(|carrier| match carrier {
            crate::types::Carrier::Quic => secure_tunnel_sdk::Carrier::Quic,
            crate::types::Carrier::Wss => secure_tunnel_sdk::Carrier::Wss,
        }),
        last_quic_failure: cache.last_quic_failure.map(|reason| match reason {
            crate::types::FallbackReason::OuterPathFailure => {
                secure_tunnel_sdk::FallbackReason::OuterPathFailure
            }
            crate::types::FallbackReason::OuterQuicRejected => {
                secure_tunnel_sdk::FallbackReason::OuterQuicRejected
            }
            crate::types::FallbackReason::OuterQuicClosedEarly => {
                secure_tunnel_sdk::FallbackReason::OuterQuicClosedEarly
            }
        }),
        next_quic_probe_after_unix_seconds: cache.next_quic_probe_after_unix_seconds,
        highest_descriptor_serial: cache.highest_descriptor_serial,
    }
}

pub fn connect_report(value: &secure_tunnel_sdk::ConnectReport) -> ConnectReport {
    ConnectReport {
        selected_carrier: carrier(value.selected_carrier),
        cache_state: cache_state(value.cache_state),
        fallback_reason: value.fallback_reason.map(fallback_reason),
        attempts: value.attempts.iter().map(attempt_report).collect(),
        transport_cache: transport_cache(&value.transport_cache),
    }
}

pub fn attempt_report(value: &secure_tunnel_sdk::TransportAttemptReport) -> TransportAttemptReport {
    let (fallback, kind, message) = match &value.outcome {
        secure_tunnel_sdk::TransportAttemptOutcome::SecureReady => (None, None, None),
        secure_tunnel_sdk::TransportAttemptOutcome::Fallback { reason } => {
            (Some(fallback_reason(*reason)), None, None)
        }
        secure_tunnel_sdk::TransportAttemptOutcome::Failed { kind, message } => {
            (None, Some(format!("{kind:?}")), Some(message.clone()))
        }
    };
    TransportAttemptReport {
        carrier: carrier(value.carrier),
        source: candidate_source(value.source),
        outcome: attempt_outcome(&value.outcome),
        fallback_reason: fallback,
        failure_kind: kind,
        failure_message: message,
    }
}

fn transport_cache(value: &secure_tunnel_sdk::TransportCacheSnapshot) -> TransportCacheSnapshot {
    TransportCacheSnapshot {
        last_successful_carrier: value.last_successful_carrier.map(carrier),
        last_quic_failure: value.last_quic_failure.map(fallback_reason),
        next_quic_probe_after_unix_seconds: value.next_quic_probe_after_unix_seconds,
        highest_descriptor_serial: value.highest_descriptor_serial,
    }
}

pub const fn sdk_account_mode(value: AccountAuthMode) -> secure_tunnel_sdk::AccountAuthMode {
    match value {
        AccountAuthMode::Fresh => secure_tunnel_sdk::AccountAuthMode::Fresh,
        AccountAuthMode::Resume => secure_tunnel_sdk::AccountAuthMode::Resume,
    }
}

pub fn account_report(value: secure_tunnel_sdk::AccountAuthReport) -> AccountAuthReport {
    AccountAuthReport {
        account_id: value.account_id,
        session_context_id: value.session_context_id,
        account_context_hash: value.account_context_hash,
        freshness: account_freshness(value.freshness),
    }
}

pub fn close_report(value: &secure_tunnel_sdk::CloseReport) -> crate::types_more::CloseReport {
    crate::types_more::CloseReport {
        final_state: session_state(value.final_state),
        classification: close_classification(value.classification),
    }
}

pub fn security_artifacts(
    value: &secure_tunnel_sdk::SecureChannelArtifacts,
) -> crate::types_more::SecureChannelArtifacts {
    crate::types_more::SecureChannelArtifacts {
        handshake_hash: value.handshake_hash.clone(),
        channel_binding: value.channel_binding.clone(),
        service_static_public_key: value.service_static_public_key.clone(),
    }
}
