# AI Agent Behavioral Rules

The following rules must be followed by any AI agent working in this repository:

1. **Commit and Push after every edit**: Perform a git commit and push after every single edit you make to the codebase.
2. **Kill app before build or edit**: Before building the app or editing it, make sure you kill any running instance of the app (`taskkill /F /IM protonsearch.exe`).
3. **Use the knowledge graph first**: Query `graphify-out/graph.json` for project analysis and edit locations. Verify only the targeted source snippets before implementation. Update the graph after source or design changes. Do not enable watch mode, hooks or cloud services without an explicit request.
