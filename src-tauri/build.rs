fn main() {
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(
        tauri_build::AppManifest::new().commands(&[
            "search",
            "execute_search_result",
            "open_search",
            "hide_search",
            "open_start",
            "open_settings",
            "hide_settings",
            "load_dock_settings",
            "save_dock_settings",
            "provider_snapshot",
            "fit_dock",
            "fit_search",
            "widget_action",
            "clipboard_snapshot",
            "running_apps_snapshot",
            "running_apps_activate",
            "weather_snapshot",
            "calendar_select_source",
            "calendar_snapshot",
            "calendar_forget_source",
        ]),
    ))
    .expect("Tauri permissions and assets");
}
