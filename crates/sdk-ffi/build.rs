// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

//! Build script for `UniFFI` UDL scaffolding.

fn main() {
    if let Err(error) = uniffi::generate_scaffolding("src/secure_tunnel_sdk_ffi.udl") {
        panic!("UniFFI scaffolding should generate: {error}");
    }
}
