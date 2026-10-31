# Graph Report - protonsearch  (2026-10-31)

## Corpus Check
- 69 files · ~144,232 words
- Verdict: corpus is large enough that graph structure adds value.
- Automatic detection did not classify 13 files. The three Slint files are represented by manual semantic extraction; the remaining unsupported formats were not parsed.

## Summary
- 1449 nodes · 3148 edges · 68 communities (58 shown, 10 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 46 edges (avg confidence: 0.69)
- Graphify structural API counters: 0 input · 0 output. Assistant semantic token usage was not measured.

## Graph Freshness
- Built from commit: `4c359532`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- Launcher and Window Events
- Launcher and Window Events · HDC
- File and Content Indexing
- Launcher and Window Events · State
- Local Agent Integration
- Search Engine and Results
- Screen Capture and OCR
- Search Engine and Results · SearchResult
- Native Dock Runtime
- Windows System Providers
- Native Settings Interface
- Launcher and Window Events · HWND
- Hotkey
- Workspace Browser Prototype
- Launcher and Window Events · paint()
- Launcher and Window Events · run()
- Application Settings
- Windows Taskbar Recovery
- Benchmark Embeddings
- Widget Runtime
- Dock Layout and Profiles
- Dock Process Supervision
- Pranshulsoni.Omnisearch.Locale.En Us
- Calculator
- Desktop Work Area Reservation
- Pranshulsoni.Omnisearch.Installer
- Launcher and Window Events · start_follow_up_chat()
- Dock Redesign Validation
- Browser Indexer
- Uninstall
- Git Indexer
- Native Dock Interface
- Planned System Architecture
- Search Engine and Results · .new()
- Windows System Providers · MonitorBounds
- 2026 10 20 Interactive Dock Widgets Design
- Widgets
- Launcher and Window Events · FilterType
- Launcher and Window Events · FormState
- Migration
- Dock Layout and Profiles · FocusTimer
- Dock Layout and Profiles · DockConfig
- Dock Layout and Profiles · model.rs
- Dock Layout and Profiles · Profile
- Native Implementation
- Dock Layout and Profiles · .normalize()
- Search Engine and Results · Connection
- Agents
- Windows
- Launcher and Window Events · HBITMAP
- Search Engine and Results · Intent
- Search Engine and Results · latest_memory_session_events()
- Search Engine and Results · .search_clipboard_history()
- Readme
- Roadmap
- Launcher and Window Events · Anim
- Launcher and Window Events · filter_pill_rects()
- Search Engine and Results · enum_windows_callback()
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
- `WidgetInstance` --represents--> `Implemented safe custom metric data`  [EXTRACTED]
  protonsearch/src/dock/model.rs → docs/2.0.0/native-implementation.md
- `Implemented opt-in Windows taskbar hiding` --implemented_by--> `TaskbarVisibility`  [EXTRACTED]
  docs/2.0.0/dock-redesign-validation.md → protonsearch/src/dock/taskbar.rs
- `Retained Win32 search controller` --supervises_via--> `DockSupervisor`  [EXTRACTED]
  docs/2.0.0/native-implementation.md → protonsearch/src/dock/supervisor.rs

## Import Cycles
- 1-file cycle: `protonsearch/src/search.rs -> protonsearch/src/search.rs`

## Communities (68 total, 10 thin omitted)

### Community 0 - "Launcher and Window Events"
Cohesion: 0.02
Nodes (86): AGENT_ICON_SIZE, AI_PANEL_H, alpha_bounds(), ANIM_ACTIVE, ANIM_DURATION_SEC, apply_sort(), BADGE_H, BADGE_W (+78 more)

### Community 1 - "Launcher and Window Events · HDC"
Cohesion: 0.05
Nodes (65): badge(), badge_custom(), block_kind(), block_top_gap(), bubble_inner_w(), COLOR_KEY, draw_highlighted_text(), draw_rounded_border_and_bg() (+57 more)

### Community 2 - "File and Content Indexing"
Cohesion: 0.07
Nodes (51): add_ignored_folder_name(), app_db_path(), configured_ignored_dirs(), DEFAULT_CONFIGURED_IGNORED_DIRS, ensure_ignored_folders_table(), extract_content(), extract_content_subprocess(), extract_via_subprocess() (+43 more)

### Community 3 - "Launcher and Window Events · State"
Cohesion: 0.07
Nodes (50): ai_scroll_down(), ai_scroll_up(), animate_window(), animated_height(), clip_id_for_pin_state(), clipboard_ocr_empty_message(), close_ai_panel(), close_ai_panel_to_agent_history() (+42 more)

### Community 4 - "Local Agent Integration"
Cohesion: 0.08
Nodes (43): agent_label(), AiConfig, ALWAYS_APPROVE, appdata_db_path(), complete(), complete_agent(), complete_chat(), complete_chat_agent() (+35 more)

### Community 5 - "Search Engine and Results"
Cohesion: 0.05
Nodes (38): AppInfo, CATALOG, catalog_index_caches_lowercase_fields(), CatalogEntry, CatalogEntryIndex, DISABLE_LIVE_RESULTS, empty_scope_result(), ensure_settings_catalog_fts() (+30 more)

### Community 6 - "Screen Capture and OCR"
Cohesion: 0.06
Nodes (31): base64_encode(), bitmap_to_rgba(), capture_screen(), create_overlay_window(), crop_bitmap(), hiword(), lens_upload_html(), loword() (+23 more)

### Community 7 - "Search Engine and Results · SearchResult"
Cohesion: 0.09
Nodes (19): AnchorCategory, candidate_limit(), content_match_source(), ellipsize_chars(), extract_path_or_url(), format_timestamp_local(), get_quick_actions(), levenshtein_distance() (+11 more)

### Community 8 - "Native Dock Runtime"
Cohesion: 0.07
Nodes (20): Stable Slint row identity, DockPanelWindow, PanelItem, action(), asset(), clock(), controller_live(), host() (+12 more)

### Community 9 - "Windows System Providers"
Cohesion: 0.09
Nodes (25): Snapshot, activate_window(), Apartment, audio_snapshot(), AudioSnapshot, bitmap_icon_pixels(), close_window(), configure_window() (+17 more)

### Community 10 - "Native Settings Interface"
Cohesion: 0.08
Nodes (26): CURRENT_VERSION, DISPLAY_VERSION, DOWNLOADED_PATH, find_launcher_hwnd(), get_db_conn(), get_desktop_wallpaper_path(), ignored_folders_model(), is_newer_version() (+18 more)

### Community 11 - "Launcher and Window Events · HWND"
Cohesion: 0.11
Nodes (34): bmp_file_bytes(), capture_clipboard_bmp_bytes(), capture_clipboard_dib_bmp_bytes(), capture_clipboard_image_data(), clipboard_bitmap_byte_len(), clipboard_lock(), copy_image_to_clipboard(), copy_to_clipboard() (+26 more)

### Community 12 - "Hotkey"
Cohesion: 0.09
Nodes (25): assemble_combo(), assemble_hotkey(), builds_register_hotkey_modifiers(), canon_part(), dropdown_keys(), every_dropdown_key_parses(), hotkey_available(), hotkey_to_slots() (+17 more)

### Community 13 - "Workspace Browser Prototype"
Cohesion: 0.16
Nodes (33): animateDockPulse(), animateProfileIn(), animateResize(), animateWidgetIn(), animateWidgetOut(), animateWidgetReflow(), applyProfile(), captureWidgetPositions() (+25 more)

### Community 14 - "Launcher and Window Events · paint()"
Cohesion: 0.09
Nodes (25): centered_in_result_row(), delete_word_before(), draw_result_icon(), floor_char_boundary(), homepage_win_h(), invalidate_results_area(), invalidate_search_row(), is_content_match_source() (+17 more)

### Community 15 - "Launcher and Window Events · run()"
Cohesion: 0.09
Nodes (25): configure_hermes_llm(), create_gdi_font(), enforce_single_instance(), get_app_icon(), get_custom_monitor(), get_file_icon(), get_registered_app_path(), get_window_icon() (+17 more)

### Community 16 - "Application Settings"
Cohesion: 0.10
Nodes (24): CLIPBOARD_LOCK, appdata_settings_path(), AppSettings, default_false(), default_global_hotkey(), default_item_height(), default_query_font_family(), default_query_font_size() (+16 more)

### Community 17 - "Windows Taskbar Recovery"
Cohesion: 0.10
Nodes (10): CALLBACK, explorer_window(), Lease, lease_path(), owner_alive(), read_lease(), restore_after_exit(), taskbar_class() (+2 more)

### Community 18 - "Benchmark Embeddings"
Cohesion: 0.11
Nodes (9): benchmark_model(), load_catalog(), main(), score_result(), convert_to_ico(), generate_search_png(), main(), load_catalog() (+1 more)

### Community 19 - "Widget Runtime"
Cohesion: 0.17
Nodes (27): act(), clearTimers(), closePanel(), live(), openPanel(), outsideDetail(), positionPanel(), queuePosition() (+19 more)

### Community 20 - "Dock Layout and Profiles"
Cohesion: 0.10
Nodes (19): Implemented safe custom metric data, clean_text(), WidgetInstance, WidgetKind, Agents, .ALL, Applications, Audio (+11 more)

### Community 21 - "Dock Process Supervision"
Cohesion: 0.14
Nodes (3): DockSupervisor, layout_path(), retry_delay()

### Community 22 - "Pranshulsoni.Omnisearch.Locale.En Us"
Cohesion: 0.10
Nodes (20): Author, Copyright, Description, License, LicenseUrl, ManifestType, ManifestVersion, Moniker (+12 more)

### Community 23 - "Calculator"
Cohesion: 0.17
Nodes (20): parse_expr(), parse_power(), parse_primary(), parse_term(), parse_unary(), Token, Caret, EOF (+12 more)

### Community 24 - "Desktop Work Area Reservation"
Cohesion: 0.16
Nodes (6): Independent --dock renderer, Independent --settings process, AppBar, same_rect(), State, window_proc()

### Community 25 - "Pranshulsoni.Omnisearch.Installer"
Cohesion: 0.10
Nodes (19): DisplayName, ProductCode, Publisher, Installers, AppsAndFeaturesEntries, Architecture, InstallerSha256, InstallerUrl (+11 more)

### Community 26 - "Launcher and Window Events · start_follow_up_chat()"
Cohesion: 0.12
Nodes (17): agent_memory_context(), ensure_agent_memory_table(), extract_agent_memory_items(), format_conversation(), get_active_app_name(), local_agent_action(), local_agent_action_routes_reasoning_requests_to_agent(), LocalAgentAction (+9 more)

### Community 27 - "Dock Redesign Validation"
Cohesion: 0.14
Nodes (18): Known baseline Clippy failure, Limit: simultaneous controller/renderer kill, Browser-only sample data boundary, Historical no-Graphify repository rule, Browser-local layout/custom metric persistence, Pending native visual/recovery acceptance, Not implemented: media metadata, Durable taskbar restoration lease (+10 more)

### Community 28 - "Browser Indexer"
Cohesion: 0.22
Nodes (12): chromium_time_to_unix_micros(), get_browser_profiles(), parse_bookmarks(), parse_firefox(), parse_history(), run_browser_indexer(), start_browser_indexer(), TempFile (+4 more)

### Community 29 - "Uninstall"
Cohesion: 0.18
Nodes (6): cleanup_startup_entries(), delete_dir_with_retry(), delete_registry_value(), kill_processes(), main(), show_message()

### Community 30 - "Git Indexer"
Cohesion: 0.22
Nodes (6): find_repos_recursive(), get_git_executable(), index_single_repo(), log_git(), run_git_indexer(), start_git_indexer()

### Community 31 - "Native Dock Interface"
Cohesion: 0.15
Nodes (13): Anchored scroll-aware native panels, 82px shell / 60px content / 48px artwork, Neutral quiet glass theme, Compact native glass design contract, DockItem, DockWindow, main(), dock_equal() (+5 more)

### Community 32 - "Planned System Architecture"
Cohesion: 0.21
Nodes (12): Design: generation-tagged incremental search, Planned: proton-dock UI boundary, Planned: proton-domain, Planned: authenticated named-pipe IPC, Planned: proton-platform-windows, Planned: proton-search service, Planned: proton-storage, Baseline: Rust and windows-rs (+4 more)

### Community 33 - "Search Engine and Results · .new()"
Cohesion: 0.36
Nodes (10): ai_prefixes_work_without_space_or_prompt(), clipboard_ocr_search_matches_compacted_text(), fresh_clipboard_schema_includes_ocr_text(), normal_search_finds_clipboard_image_ocr_text(), saved_clipboard_image_ocr_is_not_returned_twice(), search_keeps_low_ranked_ocr_candidates_beyond_requested_top_k(), settings_fts_returns_search_settings_in_fast_path(), test_hybrid_search_accuracy() (+2 more)

### Community 34 - "Windows System Providers · MonitorBounds"
Cohesion: 0.23
Nodes (9): same_display(), monitor(), popup_position(), position_panel(), covers_monitor(), fullscreen_on_monitor(), MonitorBounds, monitors() (+1 more)

### Community 35 - "2026 10 20 Interactive Dock Widgets Design"
Cohesion: 0.27
Nodes (8): Design: one continuous glass dock, Demo: widget polish stylesheet, Demo: browser profile layout controller, Demo design: anchored reusable panel, Demo: browser widget library, Demo: sample interactive runtime, Planned: Work Home Presentation compositions, Planned: Proton workspace dock

### Community 36 - "Widgets"
Cohesion: 0.33
Nodes (10): Planned: curated widget ecosystem, Planned: validated renderer-neutral view DSL, Planned: Wasmtime widget worker, Planned: proton-widgets registry, Planned: typed brokered widget actions, Planned: brokered live weather provider, Planned: widget definition contract, Design: per-placement widget instance (+2 more)

### Community 37 - "Launcher and Window Events · FilterType"
Cohesion: 0.18
Nodes (11): filter_type_from_prefix(), FilterType, All, Code, Commands, Content, Files, Folders (+3 more)

### Community 38 - "Launcher and Window Events · FormState"
Cohesion: 0.18
Nodes (11): FormState, CreateFocusCategoryBlocked, CreateFocusCategoryName, CreateNoteName, CreateQuicklinkKeyword, CreateQuicklinkName, CreateQuicklinkUrl, CreateSnippetContent (+3 more)

### Community 39 - "Migration"
Cohesion: 0.24
Nodes (7): Design: adaptive widget information density, Baseline: selected-folder FTS indexer, Baseline: Win32 search controller, Rationale: preserve personal data during migration, Design evidence: Microsoft and upstream references, Design: reuse Proton Search engine, Baseline debt: destructive uninstall

### Community 40 - "Dock Layout and Profiles · FocusTimer"
Cohesion: 0.29
Nodes (3): Implemented monotonic focus timer, focus_timer_pause_resume_completion_and_reset_use_elapsed_time(), FocusTimer

### Community 41 - "Dock Layout and Profiles · DockConfig"
Cohesion: 0.33
Nodes (5): Isolated dock-layout.json, DockConfig, LayoutStore, missing_layout_loads_defaults_and_corrupt_layout_is_preserved(), saved_layout_round_trips_and_retains_previous_backup()

### Community 42 - "Dock Layout and Profiles · model.rs"
Cohesion: 0.20
Nodes (5): MAX_CONFIG_BYTES, MAX_PINS, MAX_WIDGETS, SCHEMA_VERSION, WeatherLocation

### Community 43 - "Dock Layout and Profiles · Profile"
Cohesion: 0.22
Nodes (3): AppPin, Profile, resize_and_reorder_obey_widget_constraints()

### Community 44 - "Native Implementation"
Cohesion: 0.22
Nodes (9): Implemented pinned/running app actions, Predefined controller command bridge, Existing launcher Agents integration, Planned: widget marketplace and sandbox, Planned: multi-monitor dock instances, Implemented native first-party widgets, Owned Windows provider snapshots, Implemented Work/Home/Presentation profiles (+1 more)

### Community 45 - "Dock Layout and Profiles · .normalize()"
Cohesion: 0.39
Nodes (5): alpha_appearance_migrates_once_without_losing_profile_contents(), defaults_have_three_independent_profiles(), invalid_and_duplicate_geometry_is_normalized(), malformed_weather_location_is_not_used(), unknown_profile_and_future_version_do_not_silently_migrate()

### Community 46 - "Search Engine and Results · Connection"
Cohesion: 0.31
Nodes (4): count_memory_events(), FileRow, local_day_bounds(), memory_source_summary()

### Community 47 - "Agents"
Cohesion: 0.43
Nodes (7): Baseline debt: global approvals and credentials, Planned: single desktop-input lease, Planned: durable agent job state, Planned: deliberate Search to Run action, Baseline: Hermes agent adapter seam, Planned: proton-agent-runner, Planned: execution-time scoped grants

### Community 48 - "Windows"
Cohesion: 0.43
Nodes (7): Design: AppBar work-area negotiation, Design: default Explorer coexistence, Planned gate: native tray continuity, Planned: proton-recovery, Planned: ownership-aware restore journal, Planned: independent recovery watchdog, Planned gate: reversible taskbar hiding

### Community 49 - "Launcher and Window Events · HBITMAP"
Cohesion: 0.32
Nodes (7): BackBuffer, draw_cached_bmp(), load_shell_thumbnail(), prune_thumbnail_cache(), show_preview_window(), thumbnail_eviction_keeps_requested_path(), thumbnail_eviction_key()

### Community 50 - "Search Engine and Results · Intent"
Cohesion: 0.25
Nodes (8): clean_prompt(), Intent, FindFile, General, LaunchApp, WebSearch, peel_prefixes(), strip_any_prefix()

### Community 51 - "Search Engine and Results · latest_memory_session_events()"
Cohesion: 0.33
Nodes (5): format_unix_date(), latest_memory_session_events(), MemoryEventRow, normalize_event_timestamp(), session_source_summary()

### Community 52 - "Search Engine and Results · .search_clipboard_history()"
Cohesion: 0.40
Nodes (3): get_bmp_dimensions(), normalize_ocr_search_text(), ocr_text_matches_query()

### Community 53 - "Readme"
Cohesion: 0.40
Nodes (5): Retained Win32 search controller, Local indexed search layer, ProtonSearch, One event loop per process rationale, ProtonSearch launcher overview

### Community 54 - "Roadmap"
Cohesion: 0.50
Nodes (3): Planned targets: frame input resource budgets, Planned: isolated preview identity, Planned: native release acceptance gates

### Community 55 - "Launcher and Window Events · Anim"
Cohesion: 0.40
Nodes (5): Anim, Appearing, Hidden, Hiding, Visible

### Community 56 - "Launcher and Window Events · filter_pill_rects()"
Cohesion: 0.40
Nodes (5): filter_counts_for_results(), filter_counts_use_actual_search_results(), filter_index(), filter_pill_rects(), result_matches_filter()

### Community 58 - "Stack"
Cohesion: 0.83
Nodes (3): Planned gate: renderer SDK licensing, Design: Slint renderer candidate, Planned evaluation: Tauri fallback

## Knowledge Gaps
- **92 isolated node(s):** `Lens`, `Web`, `Darker`, `Light`, `NordDarker` (+87 more)
  These have ≤1 connection - possible missing edges or undocumented components. (Counts symbols only; 387 node(s) total have ≤1 connection when file, concept and rationale nodes are included.)
- **10 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `SearchResult` connect `Search Engine and Results · SearchResult` to `Launcher and Window Events`, `Search Engine and Results · .new()`, `Launcher and Window Events · State`, `Search Engine and Results`, `Search Engine and Results · Connection`, `Launcher and Window Events · run()`, `Launcher and Window Events · paint()`, `Search Engine and Results · latest_memory_session_events()`, `Search Engine and Results · .search_clipboard_history()`, `Launcher and Window Events · filter_pill_rects()`?**
  _High betweenness centrality (0.048) - this node is a cross-community bridge._
- **Are the 2 inferred relationships involving `State` (e.g. with `AppSettings` and `HermesApproval`) actually correct?**
  _`State` has 2 INFERRED edges - model-reasoned connections that need verification._
- **What connects `Lens`, `Web`, `Darker` to the rest of the system?**
  _92 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Launcher and Window Events` be split into smaller, more focused modules?**
  _Cohesion score 0.020335985853227233 - nodes in this community are weakly interconnected._
- **Why does `State` connect `Launcher and Window Events · State` to `Launcher and Window Events`, `Launcher and Window Events · HDC`, `Local Agent Integration`, `Launcher and Window Events · FilterType`, `Launcher and Window Events · FormState`, `Search Engine and Results · SearchResult`, `Launcher and Window Events · HWND`, `Launcher and Window Events · paint()`, `Launcher and Window Events · run()`, `Application Settings`, `Launcher and Window Events · HBITMAP`, `Launcher and Window Events · Anim`, `Launcher and Window Events · filter_pill_rects()`, `Launcher and Window Events · start_follow_up_chat()`?**
  _High betweenness centrality (0.047) - this node is a cross-community bridge._
- **Should `Launcher and Window Events · HDC` be split into smaller, more focused modules?**
  _Cohesion score 0.054244306418219465 - nodes in this community are weakly interconnected._
- **Why does `SearchEngine` connect `Search Engine and Results · SearchResult` to `Launcher and Window Events`, `Search Engine and Results · .new()`, `Search Engine and Results`, `Search Engine and Results · Connection`, `Search Engine and Results · latest_memory_session_events()`, `Search Engine and Results · .search_clipboard_history()`?**
  _High betweenness centrality (0.037) - this node is a cross-community bridge._
## Scope and verification

- Graphify 0.9.99; local AST extraction plus assistant-authored semantic fragments. No external semantic API or cloud service was configured. Assistant inference is not zero-cost; token usage was not measured.
- Application source, project documents and manually extracted Slint contracts are included; assistant reference libraries, binary artwork, generated build output and credentials are excluded.
- Planned capabilities are labelled separately from implemented native behavior. A graph edge is source evidence or a labelled inference, not a runtime test.
- Some external/unresolved AST references have no source location. Slint semantics are sampled contracts, not a complete Slint AST.
- Source files with no extracted symbols: Cargo.toml, dock_core.rs, ui.rs, install.ps1. Cargo manifests are additionally processed with --cargo.
- The interactive HTML embeds graph data locally, but loads the vis-network rendering library from a CDN. It needs internet or a cached copy of that library.
- Structural extraction used no semantic API tokens. Semantic assistant token usage is not represented by Graphify cost counters. No watch process, hooks, MCP server or application changes were added.
