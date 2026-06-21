// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use crate::inner_context::NoisePublicKey;

mod generated {
    include!(concat!(env!("OUT_DIR"), "/service_static_key_obf.rs"));
}

/// Reconstructs the build-time obfuscated service Noise static public key.
///
/// This is obfuscation, not secrecy. It prevents the public key from appearing
/// as text or as its exact 32-byte value in ordinary static binary scans, while
/// still returning the public key needed for `NK1` remote-static pinning.
#[must_use]
pub fn obfuscated_service_static_public_key() -> NoisePublicKey {
    generated::decode_service_static_public_key()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_service_static_key_decodes() {
        assert_eq!(obfuscated_service_static_public_key(), [9_u8; 32]);
    }
}
