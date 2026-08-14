# Wave 3 shell-core roll-up

- Child change: `build-superdesktop-shell-core` (kept active at the user's request)
- OpenSpec strict validation: passed
- Detailed task state: 35/35 complete, `all_done`
- Evidence validator: 35 records / 35 coverage mappings, passed
- Workspace quality gates: fmt, check, tests, Clippy warnings-as-errors, dependency architecture, core platform-type boundary, and OpenSpec strict all passed
- Consumer compile fixtures: desktop, taskbar, bridge, lifecycle passed
- Implementation revision: `b5457cbc`
- Evidence revision: `bb7130c1ee3718d46ea65d0a4b4cd8ca5a50f2d2`
- Core contract manifest SHA-256: `EC7274261C39122474322A398049F8B8FD41B6AAC1AA3927E85C42B74C297C18`
- Canonical combined input SHA-256: `626C15588B08882CB523EC5B33512535589C005A9F3A9B2F45FFCE7395C19FEA`
- Core handoff SHA-256: `E37E1AE2D0FD6FE16A71C75769C195100262CE0A2BD76A1F21E720D57DC2F565`

The contract exposes owned platform-neutral identities, state/event/effect DTOs,
bridge launch/result/repair DTOs, reconciliation, settings v1, atomic persistence
orchestration, and deterministic consumer fixtures. Program task 3.1.7 remains
pending because archiving is deferred until all active changes are implemented.
