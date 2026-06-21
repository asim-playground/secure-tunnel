// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use crate::descriptor::BootstrapDescriptor;
use crate::error::{SdkError, SdkResult};
use crate::reports::{TransportCacheSnapshot, TransportCandidateReport};

pub(super) fn connect_plan_report(
    descriptor: &BootstrapDescriptor,
    cache: Option<&TransportCacheSnapshot>,
    now_unix_seconds: u64,
) -> SdkResult<Vec<TransportCandidateReport>> {
    let core_cache = cache.map(TransportCacheSnapshot::to_core);
    descriptor
        .core_descriptor()
        .connect_plan(core_cache.as_ref(), now_unix_seconds)
        .map_err(|error| SdkError::from_core(&error))
        .map(|plan| {
            plan.iter()
                .map(TransportCandidateReport::from_core)
                .collect()
        })
}
