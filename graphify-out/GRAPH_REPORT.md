# Graph Report - .  (2026-06-23)

## Corpus Check
- 11 files · ~51,158 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 106 nodes · 175 edges · 16 communities detected
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS
- Token cost: 0 input · 0 output

## God Nodes (most connected - your core abstractions)
1. `wnd_proc()` - 12 edges
2. `SearchEngine` - 7 edges
3. `run_browser_indexer()` - 6 edges
4. `trigger_icon_loading()` - 6 edges
5. `reposition()` - 5 edges
6. `tick()` - 5 edges
7. `paint()` - 5 edges
8. `try_calc()` - 5 edges
9. `run_indexer()` - 4 edges
10. `do_show()` - 4 edges

## Surprising Connections (you probably didn't know these)
- `wnd_proc()` --calls--> `do_show()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 4 → community 9_
- `wnd_proc()` --calls--> `reposition()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 4 → community 10_
- `wnd_proc()` --calls--> `paint()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 4 → community 13_
- `do_show()` --calls--> `reposition()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 9 → community 10_
- `trigger_icon_loading()` --calls--> `get_app_icon()`  [EXTRACTED]
  opensearch-os\src\main.rs → opensearch-os\src\main.rs  _Bridges community 2 → community 9_

## Communities

### Community 0 - "Community 0"
Cohesion: 0.11
Nodes (24): AnchorCategory, AppInfo, CatalogEntry, get_live_results(), get_local_ip(), mean_pool_norm(), MEMORYSTATUSEX, MetaJson (+16 more)

### Community 1 - "Community 1"
Cohesion: 0.44
Nodes (4): get_quick_actions(), SearchEngine, test_hybrid_search_accuracy(), url_encode()

### Community 2 - "Community 2"
Cohesion: 0.33
Nodes (7): Anim, get_app_icon(), load_icon_from_memory(), main(), resolve_lnk(), run(), test_antigravity_icons()

### Community 3 - "Community 3"
Cohesion: 0.46
Nodes (7): get_browser_profiles(), parse_bookmarks(), parse_firefox(), parse_history(), run_browser_indexer(), start_browser_indexer(), traverse_bookmarks()

### Community 4 - "Community 4"
Cohesion: 0.29
Nodes (8): copy_to_clipboard(), do_hide(), ease_out(), kick_debounce(), paste_from_clipboard(), start_hide(), tick(), wnd_proc()

### Community 5 - "Community 5"
Cohesion: 0.43
Nodes (6): benchmark_model(), load_catalog(), main(), Benchmark multiple small embedding models against the Windows settings catalog., Return 1 if any keyword appears in control_name, breadcrumb_path, or synonyms., score_result()

### Community 6 - "Community 6"
Cohesion: 0.7
Nodes (4): copy_directml(), copy_model(), find_directml(), main()

### Community 7 - "Community 7"
Cohesion: 0.7
Nodes (4): get_scan_folders(), read_text_file(), run_indexer(), start_indexer()

### Community 8 - "Community 8"
Cohesion: 0.83
Nodes (3): get_known_folder_path(), handle_action(), launch()

### Community 9 - "Community 9"
Cohesion: 0.5
Nodes (4): do_show(), get_file_icon(), SendHwnd, trigger_icon_loading()

### Community 10 - "Community 10"
Cohesion: 0.5
Nodes (2): reposition(), State

### Community 11 - "Community 11"
Cohesion: 0.83
Nodes (3): convert_to_ico(), generate_search_png(), main()

### Community 12 - "Community 12"
Cohesion: 0.67
Nodes (3): load_catalog(), main(), Build-time embedding pipeline. Run once to produce catalog.bin. Usage: python sc

### Community 13 - "Community 13"
Cohesion: 1.0
Nodes (3): badge(), fill(), paint()

### Community 14 - "Community 14"
Cohesion: 1.0
Nodes (2): main(), test_ico_load()

### Community 15 - "Community 15"
Cohesion: 1.0
Nodes (0): 

## Knowledge Gaps
- **14 isolated node(s):** `Anim`, `CatalogEntry`, `SearchResult`, `MetaJson`, `AnchorCategory` (+9 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **Thin community `Community 15`** (2 nodes): `export_bge_base.py`, `main()`
  Too small to be a meaningful cluster - may be noise or needs more connections extracted.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `SearchEngine` connect `Community 1` to `Community 0`?**
  _High betweenness centrality (0.033) - this node is a cross-community bridge._
- **Why does `wnd_proc()` connect `Community 4` to `Community 9`, `Community 2`, `Community 10`, `Community 13`?**
  _High betweenness centrality (0.011) - this node is a cross-community bridge._
- **Why does `State` connect `Community 10` to `Community 2`?**
  _High betweenness centrality (0.008) - this node is a cross-community bridge._
- **What connects `Anim`, `CatalogEntry`, `SearchResult` to the rest of the system?**
  _14 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.11 - nodes in this community are weakly interconnected._