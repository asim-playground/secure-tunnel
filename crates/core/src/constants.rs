// Copyright 2025 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

/// v1 protocol identifier bound into the service descriptor and Noise prologue.
pub const PROTOCOL_ID_V1: &str = "secure-tunnel-v1";

/// v1 Noise suite identifier.
pub const NOISE_SUITE_V1: &str = "Noise_NX_25519_ChaChaPoly_BLAKE2s";

/// v1 `QUIC` ALPN identifier.
pub const QUIC_ALPN_V1: &str = PROTOCOL_ID_V1;

/// v1 `WSS` subprotocol identifier.
pub const WSS_SUBPROTOCOL_V1: &str = PROTOCOL_ID_V1;

/// Maximum plaintext payload carried in one framed record before encryption.
pub const MAX_APPLICATION_PLAINTEXT_SIZE: usize = 65_519;

/// Maximum ciphertext or handshake message payload carried in one framed record.
pub const MAX_RECORD_PAYLOAD_SIZE: usize = 65_535;
