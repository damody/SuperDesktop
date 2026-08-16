## Context

Desktop file operations now provide safe built-in commands and the provider host supplies an isolation boundary. Windows shell menu extensions are untrusted COM code and cannot run in the GPUI process. The UI must render owned descriptors only and retain useful built-in commands when native enumeration is unavailable.

## Goals / Non-Goals

**Goals:** Provide bounded menu enumeration, sanitized owned descriptors, stable command tokens, invocation admission, timeouts/cancellation, provider health, and built-in fallback menus.

**Non-Goals:** Render native HMENU objects, load third-party DLLs in GPUI, reproduce owner-drawn extension UI, or bypass Windows elevation consent.

## Decisions

1. Add context-menu request/response DTOs to the provider protocol and implement dispatch in the isolated host.
2. Convert all provider output into depth/cardinality/text-bounded `CommandDescriptor` trees. GPUI never receives pointers or callbacks.
3. Stable opaque invocation tokens bind command, selection fingerprint, and host generation. Stale tokens fail closed.
4. Built-in Open, Rename, Recycle, Properties, Refresh, Sort, and New commands are always available according to selection capabilities. Native extensions are optional enrichment with a two-second terminal deadline.
5. Provider crash or timeout closes the menu enrichment path while preserving the built-in menu and shell process.

## Risks / Trade-offs

- [Some owner-drawn extensions cannot be represented] → Mark unsupported and retain built-in commands.
- [Extension enumeration hangs] → Isolate, deadline, and terminate/restart the provider host.
- [Command IDs become stale] → Bind tokens to generation and selection fingerprint.
- [Untrusted text/icons are malformed] → Sanitize size, depth, separators, labels, and icon metadata.

## Migration Plan

Add context-menu protocol variants and host dispatch, then wire desktop interaction from its former deferred action. Existing file-operation effects remain the authority for built-in mutations.

## Open Questions

None.
