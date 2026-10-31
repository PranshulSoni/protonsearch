# Proton Search knowledge graph

Generated on 31 October 2026 using [Graphify](https://github.com/Graphify-Labs/graphify) 0.9.99.
Source snapshot: `4c359532` on branch `2.0.0`. Later commits that only store these
artifacts do not change the application snapshot.

- Open `graph.html` for the interactive viewer. Its renderer loads vis-network from a CDN.
- Read `GRAPH_REPORT.md` for communities, architectural hubs, confidence and coverage limits.
- Use `graph.json` for queries and integrations: 1,449 nodes, 3,148 edges, 68 communities.
- `semantic-design.json` and `semantic-native.json` preserve the assistant-authored
  documentation and Slint fragments. Planned features are explicitly labelled.

The application source and project documentation are included. `.graphifyignore`
excludes bundled assistant guidelines, binary artwork and generated output.
Slint contracts are extracted manually because Graphify does not parse that language.
External/unresolved symbols can lack source locations. Relationships describe source
evidence and inferences, not proof that a feature runs correctly.

## Query in PowerShell

Run from the repository root. The isolated installation is outside the project:

```powershell
& "$env:USERPROFILE\.local\bin\graphify.exe" query "DockSupervisor taskbar recovery" --budget 2000
& "$env:USERPROFILE\.local\bin\graphify.exe" explain "DockSupervisor"
& "$env:USERPROFILE\.local\bin\graphify.exe" path "DockSupervisor" "TaskbarVisibility" --undirected
```

## Refresh

```powershell
uv tool install --python 3.12 graphifyy==0.9.99
& "$env:USERPROFILE\.local\bin\graphify.exe" extract . --code-only --cargo --max-workers 4
& "$env:USERPROFILE\.local\bin\graphify.exe" cluster-only . --no-label
& "$env:USERPROFILE\.local\bin\graphify.exe" export html
```

This refreshes structural code relationships and retains existing semantic nodes.
It does not re-read documentation or Slint semantics. After those files change, ask
the assistant to refresh their semantic fragments and merge them into the graph.
Keep the saved JSON when refreshing; deleting it also removes those semantic nodes.

No watcher, Git hooks, cloud service or MCP server was enabled. No application source
was changed and the app was not launched. Graph JSON, provenance, query/path traversal
and the generated HTML were checked; application tests were unnecessary for this data-only task.
