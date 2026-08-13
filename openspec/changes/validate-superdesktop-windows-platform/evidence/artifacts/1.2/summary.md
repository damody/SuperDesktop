# Wave 2 / L2 1.2 GPUI native HWND preview

- Execution mode: capability preview only; no AppBar, Shell Hook, Explorer, or work-area mutation.
- Accepted trace SHA-256: `3AEFD276A7B2B0B0434ADBF2E8773E84C9C3F1C6518545B66D24089B9F6411F2`.
- Accepted binary SHA-256: `AB011423C7E52F2CBFBD327883D21D249FBAFA0107305FC3B591BFB629BD0FE6`.
- Common Controls v6 manifest SHA-256: `98E7D280FE2F9AFB69E7B5CDCBA61C299BE0A7A82A1CEEB864C9DA38716049D8`.
- Successor input contract: `82B7EF7275CB036D9D041B9B0E5E2299062C3EB76A642FF6C5C02F1245837429`.
- Admission trace: `B057BF54B0245F03D243B490EA324529800D5438DC8F78780400339F383E1880`.

The trace proves that the HWND came from the live GPUI window, the bridge copied DPI/display/activation payloads before forwarding, and closing reached both unordered terminal signals before finalization. Callback state reached zero; the native HWND became invalid; process handles, USER objects, and GDI objects stayed within the predeclared upper-delta thresholds after executor warm-up.

The first loader failure remains preserved as corrective evidence. It produced no accepted native trace and was superseded by B-W2-1.2-007/B-W2-1.2-008.
