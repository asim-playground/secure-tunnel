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

mod cancellation;
mod client;
mod descriptor;
mod error;
mod planning;
mod ports;
mod reports;
mod session;

pub use cancellation::CancellationHandle;
pub use client::{ClientConfig, ConnectOptions, ConnectOutcome, SecureTunnelClient};
pub use descriptor::{BootstrapDescriptor, TransportPolicyConfig};
pub use error::{ConnectError, ConnectResult, SdkError, SdkErrorKind, SdkResult};
pub use reports::{
    CacheDisposition, CandidateSource, Carrier, ConnectReport, FallbackReason,
    SecureChannelArtifacts, TransportAttemptOutcome, TransportAttemptReport,
    TransportCacheSnapshot, TransportCandidateReport,
};
pub use session::{CloseReport, SecureTunnelSession, SessionState};

#[cfg(test)]
mod tests;
