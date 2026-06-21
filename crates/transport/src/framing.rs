// Copyright 2026 Asim Ihsan
//
// This Source Code Form is subject to the terms of the Mozilla Public License, v. 2.0.
// If a copy of the MPL was not distributed with this file, You can obtain one at https://mozilla.org/MPL/2.0/.
//
// SPDX-License-Identifier: MPL-2.0

use secure_tunnel_core::{ApiError, ApiResult, CarrierKind, MAX_RECORD_PAYLOAD_SIZE};

pub fn encoded_record(record: &[u8]) -> ApiResult<Vec<u8>> {
    if record.len() > MAX_RECORD_PAYLOAD_SIZE {
        return Err(ApiError::RecordTooLarge {
            actual: record.len(),
            max: MAX_RECORD_PAYLOAD_SIZE,
        });
    }

    let length = u16::try_from(record.len()).map_err(|_| ApiError::RecordTooLarge {
        actual: record.len(),
        max: MAX_RECORD_PAYLOAD_SIZE,
    })?;
    let mut out = Vec::with_capacity(2 + record.len());
    out.extend_from_slice(&length.to_be_bytes());
    out.extend_from_slice(record);
    Ok(out)
}

pub const fn validate_inbound_record(record: &[u8], carrier: CarrierKind) -> ApiResult<()> {
    if record.len() > MAX_RECORD_PAYLOAD_SIZE {
        return Err(ApiError::OuterProtocolFailure(carrier));
    }
    Ok(())
}

pub const fn validate_outbound_record(record: &[u8]) -> ApiResult<()> {
    if record.len() > MAX_RECORD_PAYLOAD_SIZE {
        return Err(ApiError::RecordTooLarge {
            actual: record.len(),
            max: MAX_RECORD_PAYLOAD_SIZE,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use secure_tunnel_core::{ApiError, CarrierKind, MAX_RECORD_PAYLOAD_SIZE};

    use super::{encoded_record, validate_inbound_record, validate_outbound_record};

    #[test]
    fn encoded_record_uses_u16_big_endian_length() {
        let encoded = encoded_record(&[0xA0, 0xB1]).unwrap_or_else(|error| {
            panic!("record should encode: {error}");
        });

        assert_eq!(encoded, vec![0x00, 0x02, 0xA0, 0xB1]);
    }

    #[test]
    fn encoded_record_rejects_oversized_payload() {
        let payload = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE + 1];
        let Err(error) = encoded_record(&payload) else {
            panic!("record should be too large");
        };

        assert_eq!(
            error,
            ApiError::RecordTooLarge {
                actual: MAX_RECORD_PAYLOAD_SIZE + 1,
                max: MAX_RECORD_PAYLOAD_SIZE,
            }
        );
    }

    #[test]
    fn inbound_record_rejects_oversized_payload() {
        let payload = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE + 1];
        let Err(error) = validate_inbound_record(&payload, CarrierKind::Wss) else {
            panic!("record should be too large");
        };

        assert_eq!(error, ApiError::OuterProtocolFailure(CarrierKind::Wss));
    }

    #[test]
    fn outbound_record_rejects_oversized_payload() {
        let payload = vec![0_u8; MAX_RECORD_PAYLOAD_SIZE + 1];
        let Err(error) = validate_outbound_record(&payload) else {
            panic!("record should be too large");
        };

        assert_eq!(
            error,
            ApiError::RecordTooLarge {
                actual: MAX_RECORD_PAYLOAD_SIZE + 1,
                max: MAX_RECORD_PAYLOAD_SIZE,
            }
        );
    }
}
