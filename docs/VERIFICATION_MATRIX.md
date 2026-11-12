# Phase 1 verification matrix

| Requirement | Status | Evidence required |
|---|---|---|
| OCR processes beyond first 400 candidates | Partial | Candidate collection regression; real indexed corpus not yet run |
| Names-only never reads content | Not verified | Policy regression |
| Preview OCR, freshness and responsiveness | Partial | Unit and frontend tests pass; native interaction pending |
| Photos scope and OCR matches | Partial | Scope, migration and OCR-filter tests pass; native end-to-end pending |
| Offline model pack with consent and checksums | Partial | Feature-gated PP download component exists; app consent, runtime integration, crash recovery and real model inference remain unverified |
| 100+ mixed fixtures and benchmarks | Not run | Reproducible corpus, timing and machine details |
| Safe migration and restart | Not verified | Upgrade and rollback tests |
| Independent review | Pending | Failure-oriented review and native evidence |

The installed local app was opened for read-only visual inspection. Baseline hardware: ASUS Zenbook S16 UM5606GA, approximately 25.4 GB RAM. The Release executable timestamp and source commit are recorded in the phase investigation.

Windows synthetic OCR extraction corpus: 100 attempts, 100 recognized, zero failures, using generated variants of two bundled images. This is not an indexing corpus and does not cover scanned PDFs, language variants, or recognition accuracy. `cargo test -p ps-core -p ps-ipc -p ps-ocr` and the 260-test desktop suite passed after integration. Image preview and OCR decoders now reject oversized images before allocating full bitmaps; over-16-MP OCR remains unsupported. Existing Photos OCR metadata is backfilled from the filename index; missing metadata remains an open case. A fresh build and native preview/search interaction remain pending.

Latest desktop suite: 265 tests passed. A browser preview using mock data showed the OCR options in Preferences and correctly marked the three external engines unavailable. No external models were installed or tested; the updated native app and onboarding remain to be checked end to end.
