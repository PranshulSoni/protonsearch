# Release gates

- G0 baseline: partial; invalid existing embedded-UI baseline and shortcut conflict recorded.
- G1 startup: documented lazy-creation hazard identified; fix/testing underway. Reported onboarding freeze not closed.
- G2 state/motion: state repairs underway; native motion acceptance pending.
- G3 native controls: UI feedback repair underway; native integrations incomplete.
- G4 feature parity, G5 full app, G6 stress/accessibility, G7 installer: not accepted.

No production release, installation or success claim is authorized by a green compile alone. Preserve user data. Rollback scoped patches independently; preserve preexisting user changes. Test profiles are disposable but not deleted automatically.

Scoped patch independently reviewed; 251 frontend tests and embedded Release build passed. Actual skip-onboarding and returning-profile lazy Home checks succeeded on this host. All gates remain incomplete because full reliability, native integration, data, motion, installer and cross-device acceptance are outstanding. See baseline for exact limits.

Reproduce build from apps/desktop: `npm run tauri build -- --no-bundle`. Output: target/release/ProtonSearch.exe. Use PROTONSEARCH_DATA_DIR pointing to a dedicated test directory when evaluating this candidate. It is not a verified production replacement.

Final candidate SHA256: `08325FA5DF92A29B61040A9D7C5A47DE5485CFF5A6FBE7459E4763CC59AF785C`. Final embedded Release build succeeded after the search-hint change. Three existing Rust warnings remain. Temporary test-profile cleanup was rejected by automatic approval review; profiles remain at the repository root.
