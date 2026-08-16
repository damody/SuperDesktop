## Context

The M0 workspace has platform-neutral shell state in `shell-core` and Windows effects in `platform-win`, but no stable boundary for optional shell providers. Completion features need shared DTOs and isolation while retaining `unsafe_code = deny`, bounded queues, deterministic tests, and offline builds.

## Goals / Non-Goals

**Goals:**

- Define a versioned, serde-compatible protocol for provider discovery and requests.
- Make deadlines, cancellation, limits, terminal outcomes, and correlation explicit.
- Run provider dispatch behind a supervised process boundary with deterministic built-in behavior.
- Keep DTO validation platform-neutral and unit-testable.

**Non-Goals:**

- Implement individual search, menu, tray, or virtual-desktop providers.
- Stabilize a public third-party ABI in this change.
- Allow provider code to render GPUI elements or mutate shell state directly.

## Decisions

1. Add `shell-provider-protocol` as a leaf crate containing only owned DTOs, validation, and JSON framing. This avoids coupling consumers to Win32 or GPUI. Adding types directly to `shell-core` was rejected because provider wire compatibility has a different evolution cadence.
2. Use newline-delimited JSON envelopes over inherited stdin/stdout for the first host transport. Frames are capped and validated before dispatch. Named pipes were considered but add lifecycle and ACL complexity without improving the initial local one-client host.
3. Every request carries protocol version, request/correlation IDs, deadline, and typed payload. Terminal responses distinguish success, unavailable, cancelled, timeout, invalid request, and provider failure.
4. Add `shell-provider-host` as a binary/library pair. The library owns bounded dispatch and cancellation state; the binary owns framing. Consumers supervise and restart the process rather than loading providers in-process.
5. Unknown protocol versions, oversized frames, expired requests, duplicate active IDs, and malformed payloads fail closed. Unknown JSON fields remain accepted within the same major version for additive compatibility.

## Risks / Trade-offs

- [JSON adds serialization overhead] → Enforce small frames and keep high-volume image bytes out of the protocol.
- [A provider can hang] → Enforce request deadlines in the host and require the caller to terminate an unresponsive host process.
- [Protocol drift] → Publish a deterministic contract manifest and compatibility fixture tests.
- [Host restart loses in-flight state] → Give every request a terminal failure and require consumers to reconcile from source state.

## Migration Plan

Add both crates without changing existing consumers, validate fixtures, then migrate each completion feature to the protocol. Removing the crates and workspace members restores the M0 topology without data migration.

## Open Questions

None. Feature-specific payload evolution is owned by the consuming child change.
