---
status: active
normative: false
supersedes: []
superseded_by: []
---

# 2026-03-15 Doc-Set Consistency Review

## Final Summary

Active v1 docs now live under stable `v1-*` filenames, superseded dated docs
now live under `backlog/docs/historical/`, and plan/task references have been
updated to make the active-versus-archival boundary explicit. Review-driven
fixes tightened fallback gating, descriptor anti-rollback rules, and
enrollment-freshness semantics.

## Checklist

- [x] Review the dated baseline docs, active plan, and open task set for v1
      contradictions.
- [x] Confirm that the repo currently exposes both a WSS-only v1 story and a
      QUIC-preferred v1 story.
- [x] Create stable active v1 docs with explicit supersession pointers.
- [x] Downgrade the dated WSS-first artifacts to historical baseline status.
- [x] Tighten device-policy wording around protocol states, lifecycle, and
      enrollment freshness.
- [x] Update plan and task text where hard dependencies or active-source
      pointers are now misleading.
- [x] Run independent review and fix any medium/high findings before
      finalizing.

## Evidence And Conclusions

- The active plan in `backlog/plans/plan-00000001_secure-channel-foundation.md`
  already assumes `QUIC` preferred and `WSS` fallback.
- The dated threat-model doc still says `QUIC` is out of scope for v1.
- The dated protocol doc still presents itself as the primary v1 source of
  truth while only specifying the `WSS` binding.
- The dated device-policy doc is directionally strong, but still leaks carrier
  wording into a policy artifact and leaves enrollment freshness too implicit.
- Review feedback identified three material follow-up fixes:
  - fallback must honor `allow_wss_fallback`
  - signed descriptor updates need anti-rollback semantics
  - `Account Authenticated (fresh)` needs an explicit expiry rule for
    enrollment
- Those follow-up fixes are now reflected in the active transport policy,
  service descriptor, device policy, and core protocol docs.

## Next Actions

- None required for this review pass beyond future backlog acceptance or
  implementation work.
