# Proton Search knowledge graph

Generated with Graphify 0.9.99 on 1 November 2026. Application source: `eb866a9`.
Graph and ledger snapshot: `29eeb93` on branch `2.0.0`.

- `graph.html`: interactive viewer; the rendering library requires internet or a cached CDN copy.
- `GRAPH_REPORT.md`: communities, evidence, confidence and current acceptance limits.
- `graph.json`: 1,500 nodes, 3,213 relationships, 77 communities.
- `semantic-*.json`: assistant-authored documentation, Slint, Luma, stage and overnight fragments.

Application source and project documents are mapped. `.graphifyignore` excludes assistant
reference libraries, binary artwork and generated outputs. Slint contracts are mapped
manually because Graphify does not parse Slint. External or unresolved symbols can lack
source locations. A build alone does not promote a planned feature to accepted behavior.

## Current checkpoint

The build and 107 selected tests passed: 10 library and 97 binary tests, with the known
Antigravity test excluded. The overnight ledger records specific normal-mode observations
of Dock, Sound, Focus and Applications. The Customize screenshot was cropped; geometry
checks are not a complete visual pass. Broad DPI, accessibility, recovery, performance
and user acceptance remain pending. No Stage 2 delivery is claimed.

The warm neutral material is styling, not refractive Liquid Glass. Live weather, media
metadata and third-party tray hosting remain incomplete. Luma publicly documents search;
profile support and comparative indexed-content efficiency remain unverified.

## Query from the repository root in PowerShell

```powershell
& "$env:USERPROFILE\.local\bin\graphify.exe" query "native dock observed overnight acceptance" --budget 2000
& "$env:USERPROFILE\.local\bin\graphify.exe" explain "DockSupervisor"
& "$env:USERPROFILE\.local\bin\graphify.exe" path "DockSupervisor" "TaskbarVisibility" --undirected
```

## Refresh structural data

```powershell
uv tool install --python 3.12 graphifyy==0.9.99
& "$env:USERPROFILE\.local\bin\graphify.exe" extract . --code-only --cargo --max-workers 4
& "$env:USERPROFILE\.local\bin\graphify.exe" cluster-only . --no-label
& "$env:USERPROFILE\.local\bin\graphify.exe" export html
```

Keep the saved graph JSON while refreshing: existing semantic nodes are retained.
Documentation and Slint semantics require assistant fragment refresh and merge after
changes. Verify targeted current source before editing; graph provenance is a navigation
aid, not a substitute for source checks.

No watcher, Git hooks, cloud service or MCP server was enabled. Graph generation changed
no application source and launched no app. Structural counters do not measure assistant
semantic token usage. No measured speedup or zero-cost claim is made.
