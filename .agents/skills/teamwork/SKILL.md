---
name: teamwork
description: Use for complex, high-reliability software work that needs multi-agent orchestration, dual-track execution (implementation vs validation), adversarial review and independent forensic auditing before any success claim. Triggers include multi-phase plans, native/relaunch verification, "verify everything", "audit the diff", large refactors, or any task where a claim of success must be reproduced from scratch by another agent.
---

# Teamwork multi-agent skill

Act as the Teamwork coordinator. The workspace is the shared memory: every
decision that must survive a context handover is written to `.agents/teamwork/`,
not kept in the conversation.

## Step 1 - Intake and interview

1. If the request begins with `[skip-interview]`, strip the tag and go to Step 2.
2. Otherwise clarify goal, non-goals, edge cases and the acceptance evidence before
   touching code. If the user is unavailable, write the assumptions into
   `ORIGINAL_REQUEST.md` and mark them as assumptions (do not present them as facts).

## Step 2 - Workspace and stigmergic setup

1. Create `.agents/teamwork/`.
2. Write `ORIGINAL_REQUEST.md`: the validated goal, constraints, non-goals and the
   user's exact words where they are the acceptance criterion.
3. Write `PROJECT.md`: milestones 1..N, a checkbox acceptance list per milestone,
   the verification commands, and the current HEAD + `git status` snapshot.
4. Update `PROJECT.md` at every milestone boundary. A successor must be able to
   resume from these two files alone.

## Step 3 - Dual-track execution

- **Testing track**: author end-to-end and adversarial tests before or alongside
  code. Tests that cannot run in this environment must be recorded as blocked,
  never trusted.
- **Exploration track**: dispatch read-only investigation (file:line root cause,
  dependency map) before implementation. Browser-preview guesses are not native evidence.
- **Workers**: one task at a time per lane; declare the write scope before editing;
  hot files (see the repo plan) run alone. No dead code, no unstyled fallbacks,
  no speculative abstraction.

## Step 4 - Triple gate (every milestone)

1. **Review gate** (reviewer role): code cleanliness, interface conformance, style,
   smallest-diff check. Output APPROVE or numbered items with `file:line`.
2. **Challenger gate** (critic role): edge cases, stress, regressions, and a re-read
   of the acceptance list. Attempt to falsify the claim.
3. **Forensic audit** (auditor role, must be a fresh context):
   - run the build and test commands from scratch in the real workspace;
   - confirm tests genuinely execute and pass (no skipped suites, no mocked-out
     assertions, counts match);
   - verify `git diff` contains only intentional changes and that no source file
     changed during verification;
   - verdict is CLEAN or a numbered failure list. Victory is declared only on CLEAN.

## Step 5 - Succession and completion

- If context exceeds ~60% or the spawn count reaches 16, stop, summarize state in
  `PROJECT.md`, and hand over cleanly.
- Record phase evidence in `docs/qa/<phase>.md` (commands, outputs, screenshots,
  blockers). Never mark a native gate verified without an observation; write the
  exact blocker instead.

## Repo-specific gates (ProtonDock, branch `dock`)

Authority: `CLAUDE.md`, `AGENTS.md`, `docs/superpowers/plans/2026-11-08-protondock-plan.md`.
Kill `protondock.exe` and `protonsearch.exe` before any build. Never `cargo run`;
launch builds from a `%TEMP%` copy (WebView2 `0x800700AA`).

Phase A / every IC:

```
node scripts/html-pin.mjs
npx tsc -b
npm test -- --reporter=dot
npm run widgets:validate
npm run build
cargo test --manifest-path src-tauri/Cargo.toml -- --skip local_calendar::tests::
cargo test --manifest-path src-tauri/Cargo.toml local_calendar::tests::
cargo test --manifest-path protonsearch/Cargo.toml --lib weather
npx tauri build --debug --no-bundle
```

Known environment facts: 10 pre-existing `local_calendar` fixture failures
(access denied, `src-tauri/src/local_calendar.rs:465`); `ai_usage` rollover test is
flaky across a minute boundary (rerun once, then inject a fixed `now`); Computer Use
needs the `orca` CLI, which is not installed here, so native gates stay unverified.

Guardrails: never commit unrelated worktree changes; commit by pathspec and push to
`origin dock`; preserve user data; never fabricate metrics or verification.
