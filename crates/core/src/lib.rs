// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Transport-agnostic core types for Secure Tunnel v1.
//!
//! The repository is still converging on the final crate split, so this crate
//! temporarily hosts the first shared API surface for descriptor loading,
//! transport planning, session states, and transport-neutral framed I/O.

mod codec;
mod compat;
mod constants;
mod descriptor;
mod error;
mod noise;
mod selector;
mod session;
mod transport;
mod trust;

pub use compat::{ParseError, parse};
pub use constants::{
    MAX_APPLICATION_PLAINTEXT_SIZE, MAX_RECORD_PAYLOAD_SIZE, NOISE_SUITE_V1, PROTOCOL_ID_V1,
    QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1,
};
pub use descriptor::{
    CarrierSet, QuicTarget, SelectionPolicy, ServiceDescriptor, TrustAnchor, WssTarget,
    example_service_descriptor,
};
pub use error::{ApiError, ApiResult};
pub use noise::{NoiseFramedDuplex, SnowNxClientEvaluator};
pub use selector::{
    SecureReadyEvaluator, SelectedTransport, TransportAttemptOutcome, TransportAttemptTrace,
    TransportConnectors, TransportSelectionError, TransportSelector,
};
pub use session::{
    CacheDisposition, CloseDirective, SecureReadyArtifacts, SecureReadyReport,
    SecureReadyTransport, SessionPhase,
};
pub use transport::{
    BoxFuture, CandidateSource, CarrierConnector, CarrierKind, FallbackReason, FramedDuplex,
    TransportCacheSnapshot, TransportCandidate, TransportTarget,
};
pub use trust::ServerKeyAuthorizationV1;

/// Returns the stable v1 protocol identifier.
#[must_use]
pub const fn protocol_id_v1() -> &'static str {
    PROTOCOL_ID_V1
}
