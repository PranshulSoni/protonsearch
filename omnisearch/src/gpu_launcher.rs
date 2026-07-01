// GPU launcher spike (branch: gpu-test).
//
// Proof-of-concept Slint launcher rendered on the GPU (winit + femtovg by default).
// This is a SEPARATE binary target. It does NOT touch the working GDI launcher in
// main.rs. Milestone 1 only proves the visual foundation: a frameless, transparent,
// always-on-top popup with a search bar + result rows + native height animation,
// driven by static data. Hotkey, tray, focus-grab and real search wiring come later.
//
// ponytail: inline slint! macro (like src/bin/test_slint.rs) instead of a .slint file
// + build.rs change, so the settings.slint codegen path is left byte-for-byte untouched.
// Upgrade path: once the spike proves out, move this UI into ui/launcher.slint and
// switch build.rs to a single root .slint that re-exports both windows.

slint::slint! {
    struct ResultItem {
        title: string,
        subtitle: string,
        source: string,
    }

    export component LauncherWindow inherits Window {
        no-frame: true;
        background: transparent;
        always-on-top: true;
        width: 720px;
        height: 520px;

        in property <[ResultItem]> results;
        in-out property <int> current-index: 0;
        callback activated(int);
        callback query-edited(string);

        VerticalLayout {
            alignment: start;

            // ── Floating glass panel ────────────────────────────────────────
            Rectangle {
                border-radius: 18px;
                background: #1c1c1eee;          // ~93% opaque dark; corners must show desktop
                border-width: 1px;
                border-color: #ffffff24;
                drop-shadow-color: #000000a6;
                drop-shadow-blur: 34px;
                drop-shadow-offset-y: 10px;

                VerticalLayout {
                    padding: 10px;
                    spacing: 4px;

                    // Search bar
                    Rectangle {
                        height: 52px;
                        Text {
                            x: 12px;
                            y: (parent.height - self.height) / 2;
                            text: "\u{1F50D}";
                            font-size: 19px;
                            color: #808086;
                        }
                        search := TextInput {
                            x: 48px;
                            width: parent.width - 48px - 16px;
                            height: parent.height;
                            vertical-alignment: center;
                            font-size: 19px;
                            color: #f2f2f4;
                            text: "documents";
                            edited => { root.query-edited(self.text); }
                        }
                    }

                    Rectangle { height: 1px; background: #ffffff14; }

                    // Result rows
                    for item[idx] in root.results: Rectangle {
                        height: 52px;
                        border-radius: 10px;
                        background: idx == root.current-index ? #ffffff1f : transparent;

                        TouchArea {
                            clicked => {
                                root.current-index = idx;
                                root.activated(idx);
                            }
                        }

                        // icon placeholder (source tag) — real HICON->Image comes later
                        Rectangle {
                            x: 12px;
                            width: 34px;
                            height: 34px;
                            y: (parent.height - self.height) / 2;
                            border-radius: 8px;
                            background: #3a3a40;
                            Text {
                                width: parent.width;
                                height: parent.height;
                                text: item.source;
                                font-size: 9px;
                                color: #c0c0c6;
                                horizontal-alignment: center;
                                vertical-alignment: center;
                            }
                        }
                        Text {
                            x: 56px;
                            y: 9px;
                            width: parent.width - 56px - 16px;
                            text: item.title;
                            font-size: 15px;
                            color: #f0f0f2;
                            overflow: elide;
                        }
                        Text {
                            x: 56px;
                            y: 28px;
                            width: parent.width - 56px - 16px;
                            text: item.subtitle;
                            font-size: 12px;
                            color: #97979d;
                            overflow: elide;
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    let app = LauncherWindow::new().expect("create LauncherWindow");

    let rows = [
        ("Quarterly Report.docx", "Documents > Work", "DOC"),
        ("main.rs", "omnisearch > src", "CODE"),
        ("Display Settings", "Settings > System > Display", "SET"),
        ("github.com/slint-ui/slint", "History > Today", "WEB"),
        ("screenshot_2026_07_01.png", "Pictures > OCR text", "OCR"),
        ("Restart Explorer", "System > Process", "ACT"),
    ];
    let items: Vec<ResultItem> = rows
        .iter()
        .map(|(t, s, src)| ResultItem {
            title: (*t).into(),
            subtitle: (*s).into(),
            source: (*src).into(),
        })
        .collect();
    app.set_results(slint::ModelRc::new(slint::VecModel::from(items)));

    app.on_activated(|i| eprintln!("[gpu-launcher] activated row {i}"));
    app.on_query_edited(|q| eprintln!("[gpu-launcher] query: {q}"));

    app.run().expect("run event loop");
}
