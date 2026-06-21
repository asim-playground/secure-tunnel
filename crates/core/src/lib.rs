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

mod account_session;
mod app_message;
mod codec;
mod constants;
mod descriptor;
mod descriptor_auth;
mod descriptor_examples;
#[cfg(test)]
mod descriptor_tests;
mod device_session;
mod error;
#[cfg(test)]
mod fixture_tests;
mod inner_context;
mod noise;
#[cfg(test)]
mod prototype_transport;
mod selector;
mod service_key;
mod session;
mod transport;
mod trust;

pub use account_session::{
    AccountAuthMode, AccountAuthRequest, AccountAuthResult, AccountFreshness,
};
pub use app_message::{
    APP_MESSAGE_VERSION_V1, ApplicationMessage, FAMILY_ACCOUNT, FAMILY_DEVICE_AUTH,
    FAMILY_DEVICE_ENROLLMENT, TYPE_ACCOUNT_AUTH_REQUEST, TYPE_ACCOUNT_AUTH_RESULT,
    TYPE_DEVICE_AUTH_CHALLENGE, TYPE_DEVICE_AUTH_FINISH, TYPE_DEVICE_AUTH_RESULT,
    TYPE_DEVICE_AUTH_START, TYPE_DEVICE_ENROLL_CHALLENGE, TYPE_DEVICE_ENROLL_FINISH,
    TYPE_DEVICE_ENROLL_RESULT, TYPE_DEVICE_ENROLL_START,
};
pub use constants::{
    MAX_APPLICATION_PLAINTEXT_SIZE, MAX_RECORD_PAYLOAD_SIZE, NOISE_SUITE_V1, PROTOCOL_ID_V1,
    QUIC_ALPN_V1, WSS_SUBPROTOCOL_V1,
};
pub use descriptor::{
    CarrierSet, DescriptorSignature, QuicTarget, SelectionPolicy, ServiceDescriptor, TrustAnchor,
    WssTarget,
};
pub use descriptor_examples::{example_descriptor_trust_anchors, example_service_descriptor};
pub use device_session::{
    DEVICE_PROOF_PURPOSE_KNOWN_DEVICE_REAUTH, DEVICE_PROOF_PURPOSE_NEW_DEVICE_ENROLLMENT,
    DeviceAuthStart, DeviceChallenge, DeviceEnrollmentStart, DeviceProofFinish, DeviceProofInput,
    DeviceProofPurpose, DeviceResult, DeviceState, verify_device_proof_signature,
};
pub use error::{ApiError, ApiResult};
pub use inner_context::{
    DEVICE_PROOF_DOMAIN_V1, DescriptorHash, Hash32, INNER_PROTOCOL_VERSION_V1, InnerChannelContext,
    NoisePublicKey, PRODUCT_LABEL_V1, PROLOGUE_DOMAIN_V1,
};
pub use noise::{NoiseFramedDuplex, SnowNk1ClientEvaluator};
pub use selector::{
    SecureReadyEvaluator, SelectedTransport, TransportAttemptOutcome, TransportAttemptTrace,
    TransportConnectors, TransportSelectionError, TransportSelector,
};
pub use service_key::obfuscated_service_static_public_key;
pub use session::{
    CacheDisposition, CloseDirective, SecureReadyArtifacts, SecureReadyReport,
    SecureReadyTransport, SessionPhase,
};
pub use transport::{
    BoxFuture, CandidateSource, CarrierConnector, CarrierKind, FallbackReason, FramedDuplex,
    TransportCacheSnapshot, TransportCandidate, TransportTarget,
};
/// Returns the stable v1 protocol identifier.
#[must_use]
pub const fn protocol_id_v1() -> &'static str {
    PROTOCOL_ID_V1
}
