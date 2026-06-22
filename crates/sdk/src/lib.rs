// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Product SDK facade for Secure Tunnel.
//!
//! This crate is the stable Rust-facing surface that future Swift, Kotlin,
//! Python, Flutter, and Go bindings should call. It keeps the protocol and
//! transport core behind owned SDK records, opaque client/session objects, and
//! explicit SDK errors.

#![allow(clippy::module_name_repetitions, clippy::redundant_pub_crate)]

mod auth;
mod cancellation;
mod client;
mod deadline;
mod descriptor;
mod error;
mod observability;
mod planning;
mod ports;
mod reports;
mod session;

pub use auth::{
    AccountAuthMode, AccountAuthReport, AccountAuthRequest, AccountFreshness, DeviceAuthChallenge,
    DeviceAuthReport, DeviceEnrollmentChallenge, DeviceEnrollmentReport, DeviceState,
};
pub use cancellation::CancellationHandle;
pub use client::{
    ClientConfig, ConnectOptions, ConnectOutcome, HttpProxyConfig, SecureTunnelClient,
};
pub use descriptor::{BootstrapDescriptor, TransportPolicyConfig};
pub use error::{ConnectError, ConnectResult, SdkError, SdkErrorKind, SdkResult};
pub use observability::{
    AuthStage, CloseClassification, FailureClass, TelemetryEvent, TelemetryOutcome, event_names,
    metric_names,
};
pub use reports::{
    CacheDisposition, CandidateSource, Carrier, ConnectReport, FallbackReason,
    SecureChannelArtifacts, TransportAttemptOutcome, TransportAttemptReport,
    TransportCacheSnapshot, TransportCandidateReport,
};
pub use session::{CloseReport, SecureTunnelSession, SessionState};

#[cfg(test)]
mod observability_tests;
#[cfg(test)]
mod tests;
