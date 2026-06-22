---
status: active
normative: true
supersedes: []
superseded_by: []
---

# Security Hardening And STRIDE Review

## Summary

Secure Tunnel treats the inner Noise channel as the confidentiality and
integrity boundary. This hardening pass adds the availability and automation
controls needed around that boundary:

- bounded connect, carrier setup, secure-ready, record read, and record write
  operations
- cooperative cancellation that interrupts pending transport selection
- resource-exhaustion tests for stalled peers and oversized records
- explicit security-testing tasks, including cargo-mutants discovery for
  security-critical Rust files
- a STRIDE checklist for future SDK, server, and release work

The primary issue addressed is CWE-style uncontrolled resource consumption:
the client could await a malicious or degraded carrier path indefinitely,
delaying fallback and ignoring cooperative cancellation while pending.

## References

- [CWE-400 Uncontrolled Resource Consumption](https://cwe.mitre.org/data/definitions/400.html)
- [OWASP Denial of Service Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Denial_of_Service_Cheat_Sheet.html)
- [OWASP WebSocket Security Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/WebSocket_Security_Cheat_Sheet.html)
- [RFC 9000 QUIC](https://datatracker.ietf.org/doc/html/rfc9000)
- [cargo-mutants README](https://github.com/sourcefrog/cargo-mutants)
- [cargo-mutants user guide](https://mutants.rs/)

## STRIDE Matrix

| Area | Spoofing | Tampering | Repudiation | Information Disclosure | Denial Of Service | Elevation Of Privilege |
|---|---|---|---|---|---|---|
| Descriptor bootstrap | Root signature and trust-anchor checks | Canonical descriptor hash and serial rollback checks | Descriptor validation events | Redact roots, pins, raw hostnames where not configured | Reject expired, rollback, and malformed descriptors before dialing | Do not let descriptor carrier data override inner trust |
| Carrier selection | Descriptor-owned target identity | Fallback cache stores only coarse posture | Attempt/fallback events | No endpoint secrets or payloads in logs | Overall connect timeout, per-candidate timeout, cancellation | No fallback after inner trust/noise failure |
| QUIC carrier | TLS/SNI/ALPN verification | Frame size validation | Carrier attempt telemetry | No Noise payload logging | DNS/connect/open/read/write budgets; deployment Retry/address validation | QUIC rejection remains outer policy only |
| WSS carrier | TLS/subprotocol validation | Binary-only record validation | Carrier attempt telemetry | No WebSocket payload logging | Handshake/read/write budgets; slow peer tests | WSS fallback does not change identity policy |
| Noise secure-ready | Service static key pin/authorization | Prologue binds descriptor context | Secure-ready event | Handshake hash only explicit artifact, never routine log | Secure-ready timeout per candidate | Account/device auth only after secure-ready |
| Account/device protocol | Device key identity checked after account auth | Canonical challenge bytes | Auth events are staged and redacted | No credential payloads in telemetry | Payload limits and bounded record I/O | Resumed account cannot enroll new device |
| FFI/SDK bindings | Narrow opaque client/session objects | Generated binding drift checks | Package smoke reports | Owned bytes/strings only; no pointer logging | Timeout policy exposed to generated clients | FFI crates keep unsafe narrow and audited |
| CLI/FastAPI fixture | Test-only fixture metadata marked | Rust owns key custody and descriptor signing | JSON stdout plus tracing stderr | No private keys or payloads in health routes | Startup/shutdown timeouts and stderr backpressure tests | Fixture is not production auth server |
| Release artifacts | Signed/versioned packages future task | Check stale generated files | CI artifact provenance future task | Avoid embedding secrets; public key obfuscation is nuisance-only | Audit/deny/mutants/fuzz gates | No package target bypasses shared Rust facade |

## Similar Failure Families

- Slowloris-style peers that establish a carrier but never send the next
  protocol record.
- WebSocket lifecycle cleanup/resource exhaustion issues such as
  [CVE-2022-25762](https://nvd.nist.gov/vuln/detail/CVE-2022-25762).
- Stream-reset/resource-exhaustion families such as
  [CVE-2023-44487](https://nvd.nist.gov/vuln/detail/CVE-2023-44487).
- QUIC amplification, Retry exhaustion, address-validation pressure, and UDP
  blackholes.
- Fallback-cache poisoning if cancellation, trust failures, or auth failures
  are recorded as network posture.
- Parser and binding bugs around oversized records, malformed descriptors, and
  FFI ownership.

## Automated Security Testing

`mise run security:test` is the fast hardening regression suite. It covers:

- cancellation interrupting a pending selector
- overall connect timeout bounding a pending connector
- cancellation and full-connect timeout preserving attempt traces after a
  previous `QUIC` fallback
- secure-ready timeout causing fallback from stalled `QUIC`
- `WSS` peer that completes WebSocket setup but never sends a record
- `WSS` peer that sends only control frames without extending the logical
  record-read budget
- oversized `WSS` record rejection

`mise run security:mutants-list` lists cargo-mutants candidates for the
security-critical Rust files without running the mutation campaign. This is the
default local discovery command because cargo-mutants is intentionally heavier
than normal test runs.

`mise run security:mutants-smoke` runs a small cargo-mutants shard over:

- `crates/sdk/src/client.rs`
- `crates/transport/src/quic.rs`
- `crates/transport/src/wss.rs`
- `crates/transport/src/framing.rs`

Future fuzz targets should focus on descriptor JSON decode/normalize,
canonical descriptor signing input, framed record validation, connect-plan
ordering, close directive parsing, and application record parsing.

## Implementation Rules

- Timeout and cancellation failures must be classified as availability events,
  not inner trust bypasses.
- Cancellation must not mutate fallback cache as if the network path failed.
- Fallback remains legal only for fallback-eligible outer `QUIC` failures
  before `Secure Ready`.
- Inner Noise, trust, account-auth, and device-auth failures must remain
  terminal and non-fallback.
- Routine logs and metric dimensions must stay coarse and redacted.
- Server and fixture code must bound unauthenticated handshakes, idle sessions,
  shutdown, and per-record memory use before any production deployment task.
