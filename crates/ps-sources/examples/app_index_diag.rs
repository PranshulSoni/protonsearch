//! Temporary diagnostic for task-14: icon identity collisions + duplicate rows.

use std::collections::{BTreeMap, HashMap};

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn main() {
    let cache = ps_sources::default_icon_cache_dir();
    println!("cache dir: {}", cache.display());
    let (apps, warnings) = ps_sources::apps::scan_with_icons(&cache);
    println!("apps={} warnings={}", apps.len(), warnings.len());

    // Duplicate display names.
    let mut by_name: BTreeMap<String, Vec<&ps_sources::AppEntry>> = BTreeMap::new();
    for app in &apps {
        by_name
            .entry(app.name.to_lowercase())
            .or_default()
            .push(app);
    }
    let dup_names: Vec<_> = by_name.iter().filter(|(_, v)| v.len() > 1).collect();
    println!("duplicate display names: {}", dup_names.len());
    for (name, entries) in dup_names.iter().take(25) {
        println!("  DUP {name:?}:");
        for entry in entries.iter() {
            println!(
                "    target={:?} icon={:?}",
                entry.target,
                entry.icon_path.as_deref().and_then(|p| p.rsplit('\\').next())
            );
        }
    }

    // Icon path groups: which distinct sources share one cached file?
    let mut by_icon: HashMap<String, Vec<&ps_sources::AppEntry>> = HashMap::new();
    for app in &apps {
        if let Some(icon) = &app.icon_path {
            by_icon.entry(icon.clone()).or_default().push(app);
        }
    }
    println!("unique icon paths: {}", by_icon.len());
    let shared: Vec<_> = by_icon.iter().filter(|(_, v)| v.len() > 1).collect();
    println!("icon files shared by >1 entry: {}", shared.len());
    for (icon, entries) in shared.iter().take(30) {
        println!("  SHARED {}:", icon.rsplit('\\').next().unwrap_or(icon));
        for entry in entries.iter() {
            println!("    name={:?} target={:?}", entry.name, entry.target);
        }
    }

    // Cache-key collision check: distinct target strings must have distinct keys.
    let mut key_to_targets: HashMap<(u64, u32), Vec<&str>> = HashMap::new();
    for app in &apps {
        let key = format!("{}#{}", app.target.to_lowercase(), 32u32);
        let hash = fnv1a(key.as_bytes());
        key_to_targets
            .entry((hash, 32))
            .or_default()
            .push(app.target.as_str());
    }
    let collisions: Vec<_> = key_to_targets
        .iter()
        .filter(|(_, targets)| {
            let mut unique: Vec<_> = targets.iter().collect();
            unique.sort();
            unique.dedup();
            unique.len() > 1
        })
        .collect();
    println!("cache-key collisions across distinct targets: {}", collisions.len());
    for ((hash, _), targets) in collisions.iter().take(10) {
        println!("  COLLISION {hash:016x}: {targets:?}");
    }

    // Specific QA cases.
    for needle in ["amd software", "about itunes", "google chrome", "antigravity", "word", "snipping"] {
        println!("--- {needle}");
        for app in apps.iter().filter(|app| app.name.to_lowercase().contains(needle)) {
            println!(
                "  name={:?} id={:?} target={:?} icon={:?}",
                app.name,
                app.id,
                app.target,
                app.icon_path.as_deref().and_then(|p| p.rsplit('\\').next())
            );
        }
    }
}
