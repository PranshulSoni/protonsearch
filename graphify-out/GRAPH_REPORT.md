# Graph Report - protonsearch  (2026-10-31)

## Corpus Check
- 72 files · ~147,614 words
- Verdict: corpus is large enough that graph structure adds value.
- Automatic detection does not parse Slint. Native Slint contracts are included through manually refreshed semantic fragments; other unsupported formats remain unparsed.

## Summary
- 1473 nodes · 3181 edges · 78 communities (68 shown, 10 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 47 edges (avg confidence: 0.69)
- Structural semantic-API counters: 0 input · 0 output. Assistant semantic token usage was not measured.

## Graph Freshness
- Built from commit: `88b5957c`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Launcher and Window Events
- Launcher and Window Events · HDC
- File and Content Indexing
- Local Agent Integration
- Launcher and Window Events · State
- Launcher and Window Events · HWND
- Search Engine and Results
- Search Engine and Results · search.rs
- Native Settings Interface
- Workspace Browser Prototype
- Application Settings
- Launcher and Window Events · wnd_proc_inner()
- Windows System Providers
- Screen Capture and OCR
- Benchmark Embeddings
- Widget Browser Prototype
- Hotkey
- Dock Layout and Profiles
- Application Launch Actions
- Native Dock Runtime
- Launcher and Window Events · run()
- Windows Taskbar Recovery
- Pranshulsoni.Omnisearch.Locale.En Us
- Calculator
- Dock Process Supervision
- Pranshulsoni.Omnisearch.Installer
- Launcher and Window Events · start_follow_up_chat()
- Desktop Work Area Reservation
- Launcher and Window Events · LPARAM
- Uninstall
- Browser Indexer
- Search Engine and Results · .new()
- Git Indexer
- Native Implementation
- Architecture
- Search Engine and Results · insert_memory_event()
- Native Dock Runtime · State
- Dock Redesign 2026 10 24
- 2026 10 20 Interactive Dock Widgets Design
- Widgets
- Dock Layout and Profiles · model.rs
- Launcher and Window Events · FilterType
- Launcher and Window Events · FormState
- Settings Startup
- Desktop Work Area Reservation · appbar.rs
- Migration
- Dock Redesign Validation
- Dock Layout and Profiles · FocusTimer
- Dock Layout and Profiles · DockConfig
- Dock
- Verified Luma Reference
- Dock Layout and Profiles · Profile
- Windows System Providers · IconPixels
- Search Engine and Results · CatalogEntry
- Windows System Providers · MonitorBounds
- Stage One Native Acceptance
- Dock Layout and Profiles · .normalize()
- Launcher and Window Events · HBITMAP
- Agents
- Windows
- Readme
- Staged Widget Delivery
- Search Engine and Results · Intent
- Search Engine and Results · .search_windows()
- Native Dock Runtime · update_rows()
- Search Engine and Results · .search_clipboard_history()
- Roadmap
- Launcher and Window Events · Anim
- Stack
- Widget Library
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
9. `renderPanel()` - 18 edges
10. `Token` - 18 edges

## Surprising Connections (you probably didn't know these)
- `Profile` --implements--> `Implemented Work/Home/Presentation profiles`  [EXTRACTED]
  protonsearch/src/dock/model.rs → docs/2.0.0/native-implementation.md
- `action()` --routes--> `Predefined controller command bridge`  [EXTRACTED]
  protonsearch/src/dock/mod.rs → docs/2.0.0/native-implementation.md
- `Implemented opt-in Windows taskbar hiding` --implemented_by--> `TaskbarVisibility`  [EXTRACTED]
  docs/2.0.0/dock-redesign-validation.md → protonsearch/src/dock/taskbar.rs
- `Retained Win32 search controller` --supervises_via--> `DockSupervisor`  [EXTRACTED]
  docs/2.0.0/native-implementation.md → protonsearch/src/dock/supervisor.rs
- `Independent --dock renderer` --reserves_work_area_via--> `AppBar`  [EXTRACTED]
  docs/2.0.0/native-implementation.md → protonsearch/src/dock/appbar.rs

## Import Cycles
- 1-file cycle: `protonsearch/src/search.rs -> protonsearch/src/search.rs`

## Communities (78 total, 10 thin omitted)

### Community 0 - "Launcher and Window Events"
Cohesion: 0.02
Nodes (84): AGENT_ICON_SIZE, AI_PANEL_H, alpha_bounds(), ANIM_ACTIVE, ANIM_DURATION_SEC, apply_sort(), BADGE_H, BADGE_W (+76 more)

### Community 1 - "Launcher and Window Events · HDC"
Cohesion: 0.06
Nodes (61): badge(), badge_custom(), block_kind(), block_top_gap(), bubble_inner_w(), COLOR_KEY, draw_highlighted_text(), draw_rounded_border_and_bg() (+53 more)

### Community 2 - "File and Content Indexing"
Cohesion: 0.07
Nodes (51): add_ignored_folder_name(), app_db_path(), configured_ignored_dirs(), DEFAULT_CONFIGURED_IGNORED_DIRS, ensure_ignored_folders_table(), extract_content(), extract_content_subprocess(), extract_via_subprocess() (+43 more)

### Community 3 - "Local Agent Integration"
Cohesion: 0.08
Nodes (43): agent_label(), AiConfig, ALWAYS_APPROVE, appdata_db_path(), complete(), complete_agent(), complete_chat(), complete_chat_agent() (+35 more)

### Community 4 - "Launcher and Window Events · State"
Cohesion: 0.08
Nodes (35): ai_scroll_down(), ai_scroll_up(), animated_height(), centered_in_result_row(), draw_result_icon(), ease_out(), filter_index(), filter_pill_rects() (+27 more)

### Community 5 - "Launcher and Window Events · HWND"
Cohesion: 0.09
Nodes (46): animate_window(), bmp_file_bytes(), capture_clipboard_bmp_bytes(), capture_clipboard_dib_bmp_bytes(), capture_clipboard_image_data(), clipboard_bitmap_byte_len(), clipboard_lock(), close_ai_panel() (+38 more)

### Community 6 - "Search Engine and Results"
Cohesion: 0.10
Nodes (17): AnchorCategory, candidate_limit(), content_match_source(), ellipsize_chars(), extract_path_or_url(), FileRow, format_timestamp_local(), get_quick_actions() (+9 more)

### Community 7 - "Search Engine and Results · search.rs"
Cohesion: 0.06
Nodes (28): AppInfo, CATALOG, DISABLE_LIVE_RESULTS, empty_scope_result(), fmt_conv(), get_live_results(), get_local_ip(), GetDiskFreeSpaceExW() (+20 more)

### Community 8 - "Native Settings Interface"
Cohesion: 0.08
Nodes (26): CURRENT_VERSION, DISPLAY_VERSION, DOWNLOADED_PATH, find_launcher_hwnd(), get_db_conn(), get_desktop_wallpaper_path(), ignored_folders_model(), is_newer_version() (+18 more)

### Community 9 - "Workspace Browser Prototype"
Cohesion: 0.16
Nodes (33): animateDockPulse(), animateProfileIn(), animateResize(), animateWidgetIn(), animateWidgetOut(), animateWidgetReflow(), applyProfile(), captureWidgetPositions() (+25 more)

### Community 10 - "Application Settings"
Cohesion: 0.10
Nodes (25): CLIPBOARD_LOCK, plugin_allowed(), appdata_settings_path(), AppSettings, default_false(), default_global_hotkey(), default_item_height(), default_query_font_family() (+17 more)

### Community 11 - "Launcher and Window Events · wnd_proc_inner()"
Cohesion: 0.08
Nodes (33): clip_id_for_pin_state(), clip_timestamp_from_id(), clipboard_ocr_empty_message(), close_note_editor(), compute_result_reasons(), default_homepage_results(), default_homepage_results_for_settings(), delete_word_before() (+25 more)

### Community 12 - "Windows System Providers"
Cohesion: 0.11
Nodes (18): Snapshot, activate_window(), Apartment, audio_snapshot(), AudioSnapshot, close_window(), enumerate_windows(), identity() (+10 more)

### Community 13 - "Screen Capture and OCR"
Cohesion: 0.11
Nodes (16): base64_encode(), bitmap_to_rgba(), capture_screen(), create_overlay_window(), crop_bitmap(), hiword(), lens_upload_html(), loword() (+8 more)

### Community 14 - "Benchmark Embeddings"
Cohesion: 0.11
Nodes (9): benchmark_model(), load_catalog(), main(), score_result(), convert_to_ico(), generate_search_png(), main(), load_catalog() (+1 more)

### Community 15 - "Widget Browser Prototype"
Cohesion: 0.17
Nodes (27): act(), clearTimers(), closePanel(), live(), openPanel(), outsideDetail(), positionPanel(), queuePosition() (+19 more)

### Community 16 - "Hotkey"
Cohesion: 0.13
Nodes (17): assemble_combo(), assemble_hotkey(), builds_register_hotkey_modifiers(), canon_part(), dropdown_keys(), every_dropdown_key_parses(), hotkey_available(), hotkey_to_slots() (+9 more)

### Community 17 - "Dock Layout and Profiles"
Cohesion: 0.10
Nodes (19): Implemented safe custom metric data, clean_text(), WidgetInstance, WidgetKind, Agents, .ALL, Applications, Audio (+11 more)

### Community 18 - "Application Launch Actions"
Cohesion: 0.15
Nodes (15): FOCUS_ACTIVE, get_known_folder_path(), get_mute(), get_target_window(), handle_action(), handle_window_action(), key_input(), launch() (+7 more)

### Community 19 - "Native Dock Runtime"
Cohesion: 0.11
Nodes (6): asset(), clock(), image(), kind_icon(), refresh(), timer_text()

### Community 20 - "Launcher and Window Events · run()"
Cohesion: 0.11
Nodes (21): configure_hermes_llm(), create_gdi_font(), get_app_icon(), get_file_icon(), get_registered_app_path(), get_window_icon(), import_windows_clipboard_history(), load_icon_from_dll() (+13 more)

### Community 21 - "Windows Taskbar Recovery"
Cohesion: 0.16
Nodes (9): explorer_window(), Lease, lease_path(), owner_alive(), read_lease(), restore_after_exit(), taskbar_class(), TaskbarVisibility (+1 more)

### Community 22 - "Pranshulsoni.Omnisearch.Locale.En Us"
Cohesion: 0.10
Nodes (20): Author, Copyright, Description, License, LicenseUrl, ManifestType, ManifestVersion, Moniker (+12 more)

### Community 23 - "Calculator"
Cohesion: 0.17
Nodes (20): parse_expr(), parse_power(), parse_primary(), parse_term(), parse_unary(), Token, Caret, EOF (+12 more)

### Community 24 - "Dock Process Supervision"
Cohesion: 0.16
Nodes (3): DockSupervisor, layout_path(), retry_delay()

### Community 25 - "Pranshulsoni.Omnisearch.Installer"
Cohesion: 0.10
Nodes (19): DisplayName, ProductCode, Publisher, Installers, AppsAndFeaturesEntries, Architecture, InstallerSha256, InstallerUrl (+11 more)

### Community 26 - "Launcher and Window Events · start_follow_up_chat()"
Cohesion: 0.12
Nodes (17): agent_memory_context(), ensure_agent_memory_table(), extract_agent_memory_items(), format_conversation(), get_active_app_name(), local_agent_action(), local_agent_action_routes_reasoning_requests_to_agent(), LocalAgentAction (+9 more)

### Community 27 - "Desktop Work Area Reservation"
Cohesion: 0.19
Nodes (4): AppBar, same_rect(), State, window_proc()

### Community 28 - "Launcher and Window Events · LPARAM"
Cohesion: 0.18
Nodes (9): do_show(), enforce_single_instance(), get_custom_monitor(), monitor_enum_proc(), MutexHandle, preview_wnd_proc(), raise_timer_res(), UiRunCallbacks (+1 more)

### Community 29 - "Uninstall"
Cohesion: 0.17
Nodes (6): cleanup_startup_entries(), delete_dir_with_retry(), delete_registry_value(), kill_processes(), main(), show_message()

### Community 30 - "Browser Indexer"
Cohesion: 0.23
Nodes (10): chromium_time_to_unix_micros(), get_browser_profiles(), parse_bookmarks(), parse_firefox(), parse_history(), run_browser_indexer(), start_browser_indexer(), TempFile (+2 more)

### Community 31 - "Search Engine and Results · .new()"
Cohesion: 0.26
Nodes (12): ai_prefixes_work_without_space_or_prompt(), clipboard_ocr_search_matches_compacted_text(), fresh_clipboard_schema_includes_ocr_text(), image_path_dedupe_key(), normal_search_finds_clipboard_image_ocr_text(), result_launch_dedupe_key(), saved_clipboard_image_ocr_is_not_returned_twice(), search_keeps_low_ranked_ocr_candidates_beyond_requested_top_k() (+4 more)

### Community 32 - "Git Indexer"
Cohesion: 0.22
Nodes (6): find_repos_recursive(), get_git_executable(), index_single_repo(), log_git(), run_git_indexer(), start_git_indexer()

### Community 33 - "Native Implementation"
Cohesion: 0.14
Nodes (14): Implemented pinned/running app actions, Predefined controller command bridge, Retained Win32 search controller, Existing launcher Agents integration, Local indexed search layer, Planned: widget marketplace and sandbox, Planned: multi-monitor dock instances, Implemented native first-party widgets (+6 more)

### Community 34 - "Architecture"
Cohesion: 0.21
Nodes (12): Design: generation-tagged incremental search, Planned: proton-dock UI boundary, Planned: proton-domain, Planned: authenticated named-pipe IPC, Planned: proton-platform-windows, Planned: proton-search service, Planned: proton-storage, Baseline: Rust and windows-rs (+4 more)

### Community 35 - "Search Engine and Results · insert_memory_event()"
Cohesion: 0.21
Nodes (10): count_memory_events(), ensure_memory_events_schema(), format_unix_date(), insert_memory_event(), latest_memory_session_events(), local_day_bounds(), memory_source_summary(), MemoryEventRow (+2 more)

### Community 36 - "Native Dock Runtime · State"
Cohesion: 0.22
Nodes (7): action(), controller_live(), host(), launch(), native(), run(), State

### Community 37 - "Dock Redesign 2026 10 24"
Cohesion: 0.18
Nodes (11): Anchored scroll-aware native panels, 82px shell / 60px content / 48px artwork, Neutral quiet glass theme, Independent --dock renderer, Independent --settings process, Compact native glass design contract, DockWindow, popup_position() (+3 more)

### Community 38 - "2026 10 20 Interactive Dock Widgets Design"
Cohesion: 0.27
Nodes (8): Design: one continuous glass dock, Demo: widget polish stylesheet, Demo: browser profile layout controller, Demo design: anchored reusable panel, Demo: browser widget library, Demo: sample interactive runtime, Planned: Work Home Presentation compositions, Planned: Proton workspace dock

### Community 39 - "Widgets"
Cohesion: 0.33
Nodes (10): Planned: curated widget ecosystem, Planned: validated renderer-neutral view DSL, Planned: Wasmtime widget worker, Planned: proton-widgets registry, Planned: typed brokered widget actions, Planned: brokered live weather provider, Planned: widget definition contract, Design: per-placement widget instance (+2 more)

### Community 40 - "Dock Layout and Profiles · model.rs"
Cohesion: 0.18
Nodes (5): MAX_CONFIG_BYTES, MAX_PINS, MAX_WIDGETS, SCHEMA_VERSION, WeatherLocation

### Community 41 - "Launcher and Window Events · FilterType"
Cohesion: 0.18
Nodes (11): filter_type_from_prefix(), FilterType, All, Code, Commands, Content, Files, Folders (+3 more)

### Community 42 - "Launcher and Window Events · FormState"
Cohesion: 0.18
Nodes (11): FormState, CreateFocusCategoryBlocked, CreateFocusCategoryName, CreateNoteName, CreateQuicklinkKeyword, CreateQuicklinkName, CreateQuicklinkUrl, CreateSnippetContent (+3 more)

### Community 43 - "Settings Startup"
Cohesion: 0.27
Nodes (8): cleanup_legacy_startup_entries(), format_startup_command(), LEGACY_STARTUP_RUN_VALUE_NAMES, legacy_startup_shortcut_paths(), set_registry_startup(), set_run_on_startup(), STARTUP_RUN_VALUE_NAME, sync_run_on_startup()

### Community 45 - "Migration"
Cohesion: 0.24
Nodes (7): Design: adaptive widget information density, Baseline: selected-folder FTS indexer, Baseline: Win32 search controller, Rationale: preserve personal data during migration, Design evidence: Microsoft and upstream references, Design: reuse Proton Search engine, Baseline debt: destructive uninstall

### Community 46 - "Dock Redesign Validation"
Cohesion: 0.27
Nodes (10): Known baseline Clippy failure, Limit: simultaneous controller/renderer kill, Pending native visual/recovery acceptance, Not implemented: media metadata, Durable taskbar restoration lease, 104 selected native tests passed, Implemented opt-in Windows taskbar hiding, Limit: Explorer retains reserved work area (+2 more)

### Community 47 - "Dock Layout and Profiles · FocusTimer"
Cohesion: 0.29
Nodes (3): Implemented monotonic focus timer, focus_timer_pause_resume_completion_and_reset_use_elapsed_time(), FocusTimer

### Community 48 - "Dock Layout and Profiles · DockConfig"
Cohesion: 0.33
Nodes (5): Isolated dock-layout.json, DockConfig, LayoutStore, missing_layout_loads_defaults_and_corrupt_layout_is_preserved(), saved_layout_round_trips_and_retains_previous_backup()

### Community 49 - "Dock"
Cohesion: 0.22
Nodes (8): DockItem, DockPanelWindow, PanelItem, main(), dock_equal(), panel_equal(), Shared Slint application exports, Detail panel action contract

### Community 50 - "Verified Luma Reference"
Cohesion: 0.20
Nodes (10): Firecrawl access blocked; primary fallback used, Observed: differing Luma widget counts, Luma official homepage evidence, Observed: Luma one-time pricing, Unknown: Luma profile support, Luma Bar commercial reference, Verified: Luma has a Search widget, Planned: small testable widget stages (+2 more)

### Community 51 - "Dock Layout and Profiles · Profile"
Cohesion: 0.22
Nodes (3): AppPin, Profile, resize_and_reorder_obey_widget_constraints()

### Community 52 - "Windows System Providers · IconPixels"
Cohesion: 0.33
Nodes (7): bitmap_icon_pixels(), configure_window(), icon_for_path(), icon_for_window(), IconPixels, render_icon(), shell_icon_pixels()

### Community 53 - "Search Engine and Results · CatalogEntry"
Cohesion: 0.20
Nodes (9): catalog_index_caches_lowercase_fields(), CatalogEntry, CatalogEntryIndex, ensure_settings_catalog_fts(), is_ai_provider_url(), is_native_settings_command(), is_native_settings_result(), lean_allowed() (+1 more)

### Community 54 - "Windows System Providers · MonitorBounds"
Cohesion: 0.31
Nodes (7): same_display(), monitor(), covers_monitor(), fullscreen_on_monitor(), MonitorBounds, monitors(), primary_monitor()

### Community 55 - "Stage One Native Acceptance"
Cohesion: 0.22
Nodes (9): animations_enabled(), Apple design principles adapted to native Slint, Luma baseline: verified shared bar, unknown profile comparison, Stage 1: Windows animation preference policy, Pending: Stage 1 native user acceptance, Stage 1: stable neutral glass widget contract, Stage 1: native widget feedback and motion, GlassButton (+1 more)

### Community 56 - "Dock Layout and Profiles · .normalize()"
Cohesion: 0.39
Nodes (5): alpha_appearance_migrates_once_without_losing_profile_contents(), defaults_have_three_independent_profiles(), invalid_and_duplicate_geometry_is_normalized(), malformed_weather_location_is_not_used(), unknown_profile_and_future_version_do_not_silently_migrate()

### Community 57 - "Launcher and Window Events · HBITMAP"
Cohesion: 0.28
Nodes (8): BackBuffer, draw_cached_bmp(), image_path_for_result(), load_shell_thumbnail(), prune_thumbnail_cache(), show_preview_window(), thumbnail_eviction_keeps_requested_path(), thumbnail_eviction_key()

### Community 58 - "Agents"
Cohesion: 0.43
Nodes (7): Baseline debt: global approvals and credentials, Planned: single desktop-input lease, Planned: durable agent job state, Planned: deliberate Search to Run action, Baseline: Hermes agent adapter seam, Planned: proton-agent-runner, Planned: execution-time scoped grants

### Community 59 - "Windows"
Cohesion: 0.43
Nodes (7): Design: AppBar work-area negotiation, Design: default Explorer coexistence, Planned gate: native tray continuity, Planned: proton-recovery, Planned: ownership-aware restore journal, Planned: independent recovery watchdog, Planned gate: reversible taskbar hiding

### Community 60 - "Readme"
Cohesion: 0.29
Nodes (8): Browser-only sample data boundary, Current graph-first repository workflow, Browser-local layout/custom metric persistence, Not implemented: live weather provider, HTML product study verification, Unified glass HTML design study, 2.0 planning pack (historical), HTML widget dock study

### Community 61 - "Staged Widget Delivery"
Cohesion: 0.46
Nodes (8): Staged native widget implementation plan, Planned: Stage 0 — evidence and acceptance baseline, Planned: Stage 1 — one visual and interaction contract, Planned: Stage 2 — core widgets, one group per test build, Planned: Stage 3 — application and contextual widgets, Planned: Stage 4 — profiles as a visible advantage, Planned: Stage 5 — search parity, then measured improvements, Planned: Stage 6 — taskbar replacement release gate

### Community 62 - "Search Engine and Results · Intent"
Cohesion: 0.25
Nodes (8): clean_prompt(), Intent, FindFile, General, LaunchApp, WebSearch, peel_prefixes(), strip_any_prefix()

### Community 63 - "Search Engine and Results · .search_windows()"
Cohesion: 0.29
Nodes (3): enum_windows_callback(), levenshtein_distance(), WindowInfo

### Community 64 - "Native Dock Runtime · update_rows()"
Cohesion: 0.47
Nodes (3): Stable Slint row identity, model(), update_rows()

### Community 65 - "Search Engine and Results · .search_clipboard_history()"
Cohesion: 0.40
Nodes (3): get_bmp_dimensions(), normalize_ocr_search_text(), ocr_text_matches_query()

### Community 66 - "Roadmap"
Cohesion: 0.50
Nodes (3): Planned targets: frame input resource budgets, Planned: isolated preview identity, Planned: native release acceptance gates

### Community 67 - "Launcher and Window Events · Anim"
Cohesion: 0.40
Nodes (5): Anim, Appearing, Hidden, Hiding, Visible

### Community 68 - "Stack"
Cohesion: 0.83
Nodes (3): Planned gate: renderer SDK licensing, Design: Slint renderer candidate, Planned evaluation: Tauri fallback

## Knowledge Gaps
- **103 isolated node(s):** `Lens`, `Web`, `OpenUrl`, `RunPowerShell`, `Darker` (+98 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 397 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `SearchResult` connect `Search Engine and Results` to `Launcher and Window Events`, `Search Engine and Results · .search_clipboard_history()`, `Search Engine and Results · insert_memory_event()`, `Launcher and Window Events · State`, `Search Engine and Results · search.rs`, `Application Settings`, `Launcher and Window Events · wnd_proc_inner()`, `Search Engine and Results · CatalogEntry`, `Launcher and Window Events · HBITMAP`, `Search Engine and Results · .search_windows()`, `Search Engine and Results · .new()`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `State` (e.g. with `AppSettings` and `HermesApproval`) actually correct?**
  _`State` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Lens`, `Web`, `OpenUrl` to the rest of the system?**
  _103 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Launcher and Window Events` be split into smaller, more focused modules?**
  _Cohesion score 0.020289855072463767 - nodes in this community are weakly interconnected._
- **Why does `State` connect `Launcher and Window Events · State` to `Launcher and Window Events`, `Launcher and Window Events · HDC`, `Launcher and Window Events · Anim`, `Local Agent Integration`, `Launcher and Window Events · HWND`, `Search Engine and Results`, `Launcher and Window Events · FilterType`, `Launcher and Window Events · FormState`, `Launcher and Window Events · wnd_proc_inner()`, `Application Settings`, `Launcher and Window Events · run()`, `Launcher and Window Events · HBITMAP`, `Launcher and Window Events · start_follow_up_chat()`, `Launcher and Window Events · LPARAM`?**
  _High betweenness centrality (0.034) - this node is a cross-community bridge._
- **Should `Launcher and Window Events · HDC` be split into smaller, more focused modules?**
  _Cohesion score 0.05920745920745921 - nodes in this community are weakly interconnected._
- **Why does `AppSettings` connect `Application Settings` to `Native Settings Interface`, `Launcher and Window Events · wnd_proc_inner()`, `Launcher and Window Events · State`?**
  _High betweenness centrality (0.033) - this node is a cross-community bridge._
## Current stage and evidence limits

- Stage 1 native feedback/motion source is implemented at `efbd722`; final build passed and selected tests passed (94 binary, 10 library, one known Antigravity test filtered). User visual/accessibility acceptance remains PENDING. No Stage 2 implementation is claimed.
- Luma official sources document a Search widget. Profile support and comparative indexed-content efficiency remain unverified; the graph does not claim those features are absent. Marketing claims are separate from measured native performance.
- Current repository workflow is graph-first with targeted source verification. Watch mode, hooks, MCP servers and cloud services remain disabled.
- Source snapshot is the commit above. Graph artifacts commit separately; use source hashes/provenance rather than treating their own commit as a functional change.
- Existing architecture, native and design semantic nodes are retained; new Luma, stage plan and Stage 1 fragments are merged. Rust AST provenance and Slint declaration anchors were refreshed.
- Structural parsing is local. Assistant semantic extraction has unmeasured usage; zero Graphify API counters do not mean zero assistant cost. No performance or token-savings claim was measured.
- The HTML embeds graph data but loads vis-network from a CDN, requiring network access or a cached rendering library. External/unresolved AST references can lack source locations; graph relationships are evidence/inference, not proof of native behavior.
