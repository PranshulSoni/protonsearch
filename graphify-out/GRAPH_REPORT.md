# Graph Report - protonsearch  (2026-11-01)

## Corpus Check
- 73 files · ~149,059 words
- Verdict: corpus is large enough that graph structure adds value.
- Slint is unsupported by automatic detection; its native contracts are manually represented. Other unsupported formats are not parsed.

## Summary
- 1500 nodes · 3213 edges · 77 communities (65 shown, 12 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 47 edges (avg confidence: 0.69)
- Structural API counters:0input/0output. Assistant semantic token usage was not measured.

## Graph Freshness
- Built from commit: `29eeb931`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Launcher and Window Events
- File and Content Indexing
- Local Agent Integration
- Search Engine and Results
- Screen Capture and OCR
- Launcher and Window Events · wnd_proc_inner()
- Search Engine and Results · SearchResult
- Launcher and Window Events · paint()
- Launcher and Window Events · State
- Native Settings Interface
- Hotkey
- Workspace Browser Prototype
- Application Settings
- Markdown
- Windows System Providers
- Native Dock Runtime
- Benchmark Embeddings
- Widget Browser Prototype
- Launcher and Window Events · LPARAM
- Launcher and Window Events · run()
- Dock Layout and Profiles
- Dock
- Windows Taskbar Recovery
- Pranshulsoni.Omnisearch.Locale.En Us
- Calculator
- Dock Process Supervision
- Observed Native Design Iterations
- Pranshulsoni.Omnisearch.Installer
- Desktop Work Area Reservation
- Browser Indexer
- Uninstall
- Native Implementation
- Native Dock Runtime · run()
- Git Indexer
- Architecture
- Desktop Work Area Reservation · appbar.rs
- Search Engine and Results · .new()
- Launcher and Window Events · capture_clipboard_bmp_bytes()
- Dock Layout and Profiles · DockConfig
- Dock Redesign Validation
- 2026 10 20 Interactive Dock Widgets Design
- Widgets
- Launcher and Window Events · FilterType
- Launcher and Window Events · FormState
- Migration
- Dock Layout and Profiles · FocusTimer
- Windows System Providers · IconPixels
- Verified Luma Reference
- Dock Layout and Profiles · Profile
- Native Dock Runtime · position_panel()
- Windows System Providers · MonitorBounds
- Search Engine and Results · Connection
- Agents
- Windows
- Readme
- Staged Widget Delivery
- Dock Layout and Profiles · model.rs
- Search Engine and Results · Intent
- Dock Layout and Profiles · LayoutStore
- Search Engine and Results · latest_memory_session_events()
- Launcher and Window Events · local_agent_action()
- Search Engine and Results · .search_clipboard_history()
- Roadmap
- Native Dock Runtime · update_rows()
- Launcher and Window Events · Anim
- Search Engine and Results · enum_windows_callback()
- Stack
- Widget Library
- Launcher and Window Events · SearchActionHover
- Quick Actions
- Funding
- Cargo

## God Nodes (most connected - your core abstractions)
1. `State` - 92 edges
2. `wnd_proc_inner()` - 84 edges
3. `SearchResult` - 63 edges
4. `SearchEngine` - 55 edges
5. `paint()` - 47 edges
6. `execute_selected()` - 39 edges
7. `run()` - 26 edges
8. `WidgetKind` - 22 edges
9. `Native dock design iterations — 1 November 2026` - 19 edges
10. `renderPanel()` - 18 edges

## Surprising Connections (you probably didn't know these)
- `Profile` --implements--> `Implemented Work/Home/Presentation profiles`  [EXTRACTED]
  protonsearch/src/dock/model.rs → docs/2.0.0/native-implementation.md
- `action()` --routes--> `Predefined controller command bridge`  [EXTRACTED]
  protonsearch/src/dock/mod.rs → docs/2.0.0/native-implementation.md
- `update_rows()` --implements--> `Stable Slint row identity`  [EXTRACTED]
  protonsearch/src/dock/mod.rs → docs/2.0.0/dock-redesign-validation.md
- `WidgetInstance` --represents--> `Implemented safe custom metric data`  [EXTRACTED]
  protonsearch/src/dock/model.rs → docs/2.0.0/native-implementation.md
- `Retained Win32 search controller` --supervises_via--> `DockSupervisor`  [EXTRACTED]
  docs/2.0.0/native-implementation.md → protonsearch/src/dock/supervisor.rs

## Import Cycles
- 1-file cycle: `protonsearch/src/search.rs -> protonsearch/src/search.rs`

## Communities (77 total, 12 thin omitted)

### Community 0 - "Launcher and Window Events"
Cohesion: 0.02
Nodes (94): AGENT_ICON_SIZE, agent_memory_context(), AI_PANEL_H, alpha_bounds(), ANIM_ACTIVE, ANIM_DURATION_SEC, apply_sort(), BADGE_H (+86 more)

### Community 1 - "File and Content Indexing"
Cohesion: 0.07
Nodes (51): add_ignored_folder_name(), app_db_path(), configured_ignored_dirs(), DEFAULT_CONFIGURED_IGNORED_DIRS, ensure_ignored_folders_table(), extract_content(), extract_content_subprocess(), extract_via_subprocess() (+43 more)

### Community 2 - "Local Agent Integration"
Cohesion: 0.08
Nodes (43): agent_label(), AiConfig, ALWAYS_APPROVE, appdata_db_path(), complete(), complete_agent(), complete_chat(), complete_chat_agent() (+35 more)

### Community 3 - "Search Engine and Results"
Cohesion: 0.05
Nodes (38): AppInfo, CATALOG, catalog_index_caches_lowercase_fields(), CatalogEntry, CatalogEntryIndex, DISABLE_LIVE_RESULTS, empty_scope_result(), ensure_settings_catalog_fts() (+30 more)

### Community 4 - "Screen Capture and OCR"
Cohesion: 0.06
Nodes (31): base64_encode(), bitmap_to_rgba(), capture_screen(), create_overlay_window(), crop_bitmap(), hiword(), lens_upload_html(), loword() (+23 more)

### Community 5 - "Launcher and Window Events · wnd_proc_inner()"
Cohesion: 0.06
Nodes (41): ai_scroll_down(), ai_scroll_up(), animated_height(), clip_id_for_pin_state(), clip_timestamp_from_id(), clipboard_ocr_empty_message(), compute_result_reasons(), delete_word_before() (+33 more)

### Community 6 - "Search Engine and Results · SearchResult"
Cohesion: 0.09
Nodes (19): AnchorCategory, candidate_limit(), content_match_source(), ellipsize_chars(), extract_path_or_url(), format_timestamp_local(), get_quick_actions(), levenshtein_distance() (+11 more)

### Community 7 - "Launcher and Window Events · paint()"
Cohesion: 0.08
Nodes (45): BackBuffer, badge(), badge_custom(), bubble_inner_w(), centered_in_result_row(), COLOR_KEY, create_gdi_font(), draw_cached_bmp() (+37 more)

### Community 8 - "Launcher and Window Events · State"
Cohesion: 0.10
Nodes (46): clipboard_lock(), close_ai_panel(), close_ai_panel_to_agent_history(), close_note_editor(), copy_to_clipboard(), create_agent(), do_hide(), execute_selected() (+38 more)

### Community 9 - "Native Settings Interface"
Cohesion: 0.08
Nodes (26): CURRENT_VERSION, DISPLAY_VERSION, DOWNLOADED_PATH, find_launcher_hwnd(), get_db_conn(), get_desktop_wallpaper_path(), ignored_folders_model(), is_newer_version() (+18 more)

### Community 10 - "Hotkey"
Cohesion: 0.09
Nodes (25): assemble_combo(), assemble_hotkey(), builds_register_hotkey_modifiers(), canon_part(), dropdown_keys(), every_dropdown_key_parses(), hotkey_available(), hotkey_to_slots() (+17 more)

### Community 11 - "Workspace Browser Prototype"
Cohesion: 0.16
Nodes (33): animateDockPulse(), animateProfileIn(), animateResize(), animateWidgetIn(), animateWidgetOut(), animateWidgetReflow(), applyProfile(), captureWidgetPositions() (+25 more)

### Community 12 - "Application Settings"
Cohesion: 0.10
Nodes (24): CLIPBOARD_LOCK, appdata_settings_path(), AppSettings, default_false(), default_global_hotkey(), default_item_height(), default_query_font_family(), default_query_font_size() (+16 more)

### Community 13 - "Markdown"
Cohesion: 0.11
Nodes (31): block_kind(), block_top_gap(), measure_blocks(), measure_one(), strip_inline_text(), wrap_text_height(), fence_info(), find_marker() (+23 more)

### Community 14 - "Windows System Providers"
Cohesion: 0.11
Nodes (18): Snapshot, activate_window(), Apartment, audio_snapshot(), AudioSnapshot, close_window(), enumerate_windows(), identity() (+10 more)

### Community 15 - "Native Dock Runtime"
Cohesion: 0.09
Nodes (9): DockItem, asset(), clock(), dock_content_width(), dock_equal(), image(), kind_icon(), refresh() (+1 more)

### Community 16 - "Benchmark Embeddings"
Cohesion: 0.11
Nodes (9): benchmark_model(), load_catalog(), main(), score_result(), convert_to_ico(), generate_search_png(), main(), load_catalog() (+1 more)

### Community 17 - "Widget Browser Prototype"
Cohesion: 0.17
Nodes (27): act(), clearTimers(), closePanel(), live(), openPanel(), outsideDetail(), positionPanel(), queuePosition() (+19 more)

### Community 18 - "Launcher and Window Events · LPARAM"
Cohesion: 0.13
Nodes (17): animate_window(), do_show(), enforce_single_instance(), get_active_app_name(), get_custom_monitor(), get_window_icon(), log_timeline_event(), monitor_enum_proc() (+9 more)

### Community 19 - "Launcher and Window Events · run()"
Cohesion: 0.10
Nodes (21): configure_hermes_llm(), get_app_icon(), get_file_icon(), get_registered_app_path(), import_windows_clipboard_history(), load_icon_from_dll(), load_png_to_hicon(), MutexHandle (+13 more)

### Community 20 - "Dock Layout and Profiles"
Cohesion: 0.10
Nodes (19): Implemented safe custom metric data, clean_text(), WidgetInstance, WidgetKind, Agents, .ALL, Applications, Audio (+11 more)

### Community 21 - "Dock"
Cohesion: 0.11
Nodes (20): Independent --dock renderer, Independent --settings process, DockPanelWindow, DockWindow, PanelItem, main(), animations_enabled(), panel_equal() (+12 more)

### Community 22 - "Windows Taskbar Recovery"
Cohesion: 0.17
Nodes (9): explorer_window(), Lease, lease_path(), owner_alive(), read_lease(), restore_after_exit(), taskbar_class(), TaskbarVisibility (+1 more)

### Community 23 - "Pranshulsoni.Omnisearch.Locale.En Us"
Cohesion: 0.10
Nodes (20): Author, Copyright, Description, License, LicenseUrl, ManifestType, ManifestVersion, Moniker (+12 more)

### Community 24 - "Calculator"
Cohesion: 0.17
Nodes (20): parse_expr(), parse_power(), parse_primary(), parse_term(), parse_unary(), Token, Caret, EOF (+12 more)

### Community 25 - "Dock Process Supervision"
Cohesion: 0.16
Nodes (3): DockSupervisor, layout_path(), retry_delay()

### Community 26 - "Observed Native Design Iterations"
Cohesion: 0.10
Nodes (20): Native dock design iterations — 1 November 2026, 107 selected overnight tests passed, Current44px resting app artwork, Customize geometry checked but full visual pass pending, Debug-only native inspection mode, Final overnight source and normal-mode verification, Implemented explicit native and Slint geometry agreement, Observed iteration 1 geometry defects (+12 more)

### Community 27 - "Pranshulsoni.Omnisearch.Installer"
Cohesion: 0.10
Nodes (19): DisplayName, ProductCode, Publisher, Installers, AppsAndFeaturesEntries, Architecture, InstallerSha256, InstallerUrl (+11 more)

### Community 28 - "Desktop Work Area Reservation"
Cohesion: 0.19
Nodes (4): AppBar, same_rect(), State, window_proc()

### Community 29 - "Browser Indexer"
Cohesion: 0.22
Nodes (12): chromium_time_to_unix_micros(), get_browser_profiles(), parse_bookmarks(), parse_firefox(), parse_history(), run_browser_indexer(), start_browser_indexer(), TempFile (+4 more)

### Community 30 - "Uninstall"
Cohesion: 0.17
Nodes (6): cleanup_startup_entries(), delete_dir_with_retry(), delete_registry_value(), kill_processes(), main(), show_message()

### Community 31 - "Native Implementation"
Cohesion: 0.13
Nodes (15): Implemented pinned/running app actions, Predefined controller command bridge, Retained Win32 search controller, Existing launcher Agents integration, Isolated dock-layout.json, Local indexed search layer, Planned: widget marketplace and sandbox, Planned: multi-monitor dock instances (+7 more)

### Community 32 - "Native Dock Runtime · run()"
Cohesion: 0.18
Nodes (9): action(), coalesce_volume_commands(), controller_live(), host(), launch(), native(), panel_has_foreground(), run() (+1 more)

### Community 33 - "Git Indexer"
Cohesion: 0.22
Nodes (6): find_repos_recursive(), get_git_executable(), index_single_repo(), log_git(), run_git_indexer(), start_git_indexer()

### Community 34 - "Architecture"
Cohesion: 0.21
Nodes (12): Design: generation-tagged incremental search, Planned: proton-dock UI boundary, Planned: proton-domain, Planned: authenticated named-pipe IPC, Planned: proton-platform-windows, Planned: proton-search service, Planned: proton-storage, Baseline: Rust and windows-rs (+4 more)

### Community 36 - "Search Engine and Results · .new()"
Cohesion: 0.36
Nodes (10): ai_prefixes_work_without_space_or_prompt(), clipboard_ocr_search_matches_compacted_text(), fresh_clipboard_schema_includes_ocr_text(), normal_search_finds_clipboard_image_ocr_text(), saved_clipboard_image_ocr_is_not_returned_twice(), search_keeps_low_ranked_ocr_candidates_beyond_requested_top_k(), settings_fts_returns_search_settings_in_fast_path(), test_hybrid_search_accuracy() (+2 more)

### Community 37 - "Launcher and Window Events · capture_clipboard_bmp_bytes()"
Cohesion: 0.18
Nodes (10): bmp_file_bytes(), capture_clipboard_bmp_bytes(), capture_clipboard_dib_bmp_bytes(), capture_clipboard_image_data(), clipboard_bitmap_byte_len(), copy_image_to_clipboard(), dib_clipboard_bytes_are_wrapped_as_bmp(), dib_to_bmp_file_bytes() (+2 more)

### Community 38 - "Dock Layout and Profiles · DockConfig"
Cohesion: 0.33
Nodes (6): alpha_appearance_migrates_once_without_losing_profile_contents(), defaults_have_three_independent_profiles(), DockConfig, invalid_and_duplicate_geometry_is_normalized(), malformed_weather_location_is_not_used(), WeatherLocation

### Community 39 - "Dock Redesign Validation"
Cohesion: 0.24
Nodes (11): Known baseline Clippy failure, Limit: simultaneous controller/renderer kill, Pending native visual/recovery acceptance, Not implemented: media metadata, Durable taskbar restoration lease, 104 selected native tests passed, Stable Slint row identity, Implemented opt-in Windows taskbar hiding (+3 more)

### Community 40 - "2026 10 20 Interactive Dock Widgets Design"
Cohesion: 0.27
Nodes (8): Design: one continuous glass dock, Demo: widget polish stylesheet, Demo: browser profile layout controller, Demo design: anchored reusable panel, Demo: browser widget library, Demo: sample interactive runtime, Planned: Work Home Presentation compositions, Planned: Proton workspace dock

### Community 41 - "Widgets"
Cohesion: 0.33
Nodes (10): Planned: curated widget ecosystem, Planned: validated renderer-neutral view DSL, Planned: Wasmtime widget worker, Planned: proton-widgets registry, Planned: typed brokered widget actions, Planned: brokered live weather provider, Planned: widget definition contract, Design: per-placement widget instance (+2 more)

### Community 42 - "Launcher and Window Events · FilterType"
Cohesion: 0.18
Nodes (11): filter_type_from_prefix(), FilterType, All, Code, Commands, Content, Files, Folders (+3 more)

### Community 43 - "Launcher and Window Events · FormState"
Cohesion: 0.18
Nodes (11): FormState, CreateFocusCategoryBlocked, CreateFocusCategoryName, CreateNoteName, CreateQuicklinkKeyword, CreateQuicklinkName, CreateQuicklinkUrl, CreateSnippetContent (+3 more)

### Community 44 - "Migration"
Cohesion: 0.24
Nodes (7): Design: adaptive widget information density, Baseline: selected-folder FTS indexer, Baseline: Win32 search controller, Rationale: preserve personal data during migration, Design evidence: Microsoft and upstream references, Design: reuse Proton Search engine, Baseline debt: destructive uninstall

### Community 45 - "Dock Layout and Profiles · FocusTimer"
Cohesion: 0.29
Nodes (3): Implemented monotonic focus timer, focus_timer_pause_resume_completion_and_reset_use_elapsed_time(), FocusTimer

### Community 46 - "Windows System Providers · IconPixels"
Cohesion: 0.33
Nodes (7): bitmap_icon_pixels(), configure_window(), icon_for_path(), icon_for_window(), IconPixels, render_icon(), shell_icon_pixels()

### Community 47 - "Verified Luma Reference"
Cohesion: 0.20
Nodes (10): Firecrawl access blocked; primary fallback used, Observed: differing Luma widget counts, Luma official homepage evidence, Observed: Luma one-time pricing, Unknown: Luma profile support, Luma Bar commercial reference, Verified: Luma has a Search widget, Planned: small testable widget stages (+2 more)

### Community 48 - "Dock Layout and Profiles · Profile"
Cohesion: 0.22
Nodes (3): AppPin, Profile, resize_and_reorder_obey_widget_constraints()

### Community 49 - "Native Dock Runtime · position_panel()"
Cohesion: 0.22
Nodes (8): Anchored scroll-aware native panels, Historical design:82px shell /60px content /48px artwork, Neutral quiet glass theme, Compact native glass design contract, panel_dimensions(), popup_position(), position_panel(), simple_widgets_do_not_open_editor_sized_windows()

### Community 50 - "Windows System Providers · MonitorBounds"
Cohesion: 0.31
Nodes (7): same_display(), monitor(), covers_monitor(), fullscreen_on_monitor(), MonitorBounds, monitors(), primary_monitor()

### Community 51 - "Search Engine and Results · Connection"
Cohesion: 0.31
Nodes (4): count_memory_events(), FileRow, local_day_bounds(), memory_source_summary()

### Community 52 - "Agents"
Cohesion: 0.43
Nodes (7): Baseline debt: global approvals and credentials, Planned: single desktop-input lease, Planned: durable agent job state, Planned: deliberate Search to Run action, Baseline: Hermes agent adapter seam, Planned: proton-agent-runner, Planned: execution-time scoped grants

### Community 53 - "Windows"
Cohesion: 0.43
Nodes (7): Design: AppBar work-area negotiation, Design: default Explorer coexistence, Planned gate: native tray continuity, Planned: proton-recovery, Planned: ownership-aware restore journal, Planned: independent recovery watchdog, Planned gate: reversible taskbar hiding

### Community 54 - "Readme"
Cohesion: 0.29
Nodes (8): Browser-only sample data boundary, Current graph-first repository workflow, Browser-local layout/custom metric persistence, Not implemented: live weather provider, HTML product study verification, Unified glass HTML design study, 2.0 planning pack (historical), HTML widget dock study

### Community 55 - "Staged Widget Delivery"
Cohesion: 0.46
Nodes (8): Staged native widget implementation plan, Planned: Stage 0 — evidence and acceptance baseline, Planned: Stage 1 — one visual and interaction contract, Planned: Stage 2 — core widgets, one group per test build, Planned: Stage 3 — application and contextual widgets, Planned: Stage 4 — profiles as a visible advantage, Planned: Stage 5 — search parity, then measured improvements, Planned: Stage 6 — taskbar replacement release gate

### Community 56 - "Dock Layout and Profiles · model.rs"
Cohesion: 0.25
Nodes (5): MAX_CONFIG_BYTES, MAX_PINS, MAX_WIDGETS, SCHEMA_VERSION, unknown_profile_and_future_version_do_not_silently_migrate()

### Community 57 - "Search Engine and Results · Intent"
Cohesion: 0.25
Nodes (8): clean_prompt(), Intent, FindFile, General, LaunchApp, WebSearch, peel_prefixes(), strip_any_prefix()

### Community 58 - "Dock Layout and Profiles · LayoutStore"
Cohesion: 0.43
Nodes (3): LayoutStore, missing_layout_loads_defaults_and_corrupt_layout_is_preserved(), saved_layout_round_trips_and_retains_previous_backup()

### Community 59 - "Search Engine and Results · latest_memory_session_events()"
Cohesion: 0.33
Nodes (5): format_unix_date(), latest_memory_session_events(), MemoryEventRow, normalize_event_timestamp(), session_source_summary()

### Community 60 - "Launcher and Window Events · local_agent_action()"
Cohesion: 0.33
Nodes (6): local_agent_action(), local_agent_action_routes_reasoning_requests_to_agent(), LocalAgentAction, OpenUrl, RunPowerShell, run_local_agent_action()

### Community 61 - "Search Engine and Results · .search_clipboard_history()"
Cohesion: 0.40
Nodes (3): get_bmp_dimensions(), normalize_ocr_search_text(), ocr_text_matches_query()

### Community 62 - "Roadmap"
Cohesion: 0.50
Nodes (3): Planned targets: frame input resource budgets, Planned: isolated preview identity, Planned: native release acceptance gates

### Community 64 - "Launcher and Window Events · Anim"
Cohesion: 0.40
Nodes (5): Anim, Appearing, Hidden, Hiding, Visible

### Community 66 - "Stack"
Cohesion: 0.83
Nodes (3): Planned gate: renderer SDK licensing, Design: Slint renderer candidate, Planned evaluation: Tauri fallback

### Community 69 - "Launcher and Window Events · SearchActionHover"
Cohesion: 0.67
Nodes (3): SearchActionHover, Lens, Web

## Knowledge Gaps
- **122 isolated node(s):** `Darker`, `Light`, `NordDarker`, `Agents`, `Applications` (+117 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 418 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **12 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `WidgetKind` connect `Dock Layout and Profiles` to `Dock Layout and Profiles · Profile`, `Dock Layout and Profiles · model.rs`?**
  _High betweenness centrality (0.035) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `State` (e.g. with `AppSettings` and `HermesApproval`) actually correct?**
  _`State` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Darker`, `Light`, `NordDarker` to the rest of the system?**
  _122 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Launcher and Window Events` be split into smaller, more focused modules?**
  _Cohesion score 0.019483870967741936 - nodes in this community are weakly interconnected._
- **Why does `SearchResult` connect `Search Engine and Results · SearchResult` to `Launcher and Window Events`, `Search Engine and Results`, `Search Engine and Results · .new()`, `Launcher and Window Events · wnd_proc_inner()`, `Launcher and Window Events · paint()`, `Launcher and Window Events · State`, `Search Engine and Results · Connection`, `Search Engine and Results · latest_memory_session_events()`, `Search Engine and Results · .search_clipboard_history()`?**
  _High betweenness centrality (0.032) - this node is a cross-community bridge._
- **Should `File and Content Indexing` be split into smaller, more focused modules?**
  _Cohesion score 0.07093253968253968 - nodes in this community are weakly interconnected._
- **Why does `SearchEngine` connect `Search Engine and Results · SearchResult` to `Launcher and Window Events`, `Search Engine and Results`, `Search Engine and Results · .new()`, `Search Engine and Results · Connection`, `Search Engine and Results · latest_memory_session_events()`, `Search Engine and Results · .search_clipboard_history()`?**
  _High betweenness centrality (0.028) - this node is a cross-community bridge._
## Current native checkpoint — 1 November 2026

- Final application source:`eb866a9`; ledger snapshot:`29eeb93`. Final native build58.87s passed,107selected tests(10library+97binary) passed; one known baseline Antigravity test filtered. Targeted Rust formatting passed. Clippy was not rerun; existing main.rs diagnostics remain.
- Normal-mode native inspection verified the specific Dock/Sound/Focus layouts, left-anchored volume fill, retained departing content during close, and multi-row Applications drawer with friendly labels. Prior failed viewport iterations remain historical evidence.
- Customize accessibility and DPI-aware client geometry agree; its cropped screenshot does NOT establish full visual acceptance. Full keyboard/screen-reader,DPI/mixed-monitor,fullscreen,Explorer/taskbar recovery and measured resource/frame-pacing gates remain open. User acceptance is pending. Stage2 is not claimed.
- Warm neutral translucent styling is implemented; refractive Liquid Glass is not. Live weather, media metadata and third-party tray hosting remain incomplete. Profiles, launch commands and search/index behavior are retained.
- Existing design/native/Luma/stage semantic nodes are retained. Fresh Rust AST and manual Slint declaration anchors distinguish current source from historical contracts and observations. A graph relationship is evidence or labelled inference, not proof of runtime correctness.
- Graph generation did not launch or change the app. No watcher,hooks,MCP server or cloud service was enabled. Local structural parsing uses no semantic API; assistant semantic usage was unmeasured and is not zero-cost. No performance or token-savings measurement is claimed.
- The viewer embeds graph data locally but loads vis-network from a CDN. It requires network access or a cached rendering library. External/unresolved symbols may have no source location.
