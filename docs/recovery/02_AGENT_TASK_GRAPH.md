# Agent task graph

Harness exposes gpt-6.1-sol/medium and gpt-6-luna/high. Requested Terra is unavailable. Four concurrent slots prevent simultaneous six-lead execution. No alternate model is represented as Terra.

Completed Sol analysis responsibilities: startup/lifecycle; island/motion; native integrations plus service audit. Luna implementation ownership: startup main.rs; island IslandView.tsx/island.test.tsx; controls NativeCards.tsx/island.css/islandCards.test.tsx. These owners must preserve concurrent edits. Independent review is required before acceptance; implementation workers cannot approve their own work.

Dependency: baseline → scoped repairs/tests → independent review → embedded Release build → native repro → remaining integrations → hardware/pairwise/installer acceptance. Search/clipboard, agents and security/release responsibilities still need full dedicated audits.
