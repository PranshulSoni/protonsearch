# Root cause ledger

| ID | Evidence | Required repair / verification |
|---|---|---|
| START-01 | Lazy WebviewWindowBuilder reached by synchronous commands and event callbacks; installed Tauri 2.12.5 explicitly warns of Windows deadlock on those routes | Dispatch creation outside UI callbacks; native first-open regression. Not yet attributed to reported onboarding freeze |
| START-02 | Existing release-path executable loads localhost and fails; Alt+Space registration rejected | Build through Tauri packaging; identify shortcut conflict before hotkey measurements |
| ISLAND-01 | Empty click expands; clear never collapses; fixed 397px override; tests enforce defect | Query-driven compact Search and bounded existing dimensions; regression tests |
| ISLAND-02 | 350ms shared throttle drops explicit dot selections; delayed transitions survive dismissal | Preserve latest requested target and cancel stale transitions |
| QS-01 | Drilldown returns before error rendering; rejection strings discarded | Visible pending/error/success in root and detail; rejection regression |
| QS-02 | aria-checked state has no active color style | Blue checked radio controls; visual verification |
| NATIVE-01 | Media explicitly unsupported in two layers | Real session adapter still required |
| NATIVE-02 | Brightness only uses DDC/DXVA2; no internal-panel WMI path | Supported laptop brightness remains open |
| NATIVE-03 | Controls snapshot prefetches once; external OS changes stale | Event/revisit refresh remains open |

Existing memory-target no-op is a user change, not verified root-cause evidence. Engine opening before windows and synchronous shutdown joins warrant separate measurement.
