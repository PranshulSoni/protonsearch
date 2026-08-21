use anyhow::{Context, Result};
use protonsearch_linux::{
    actions, capabilities, desktop, gui, hyprland, platform, providers, search, settings, system,
    update, xdg,
};
use std::io::Read;
use std::path::PathBuf;

fn main() -> Result<()> {
    refuse_root_and_harden_process();
    let paths = xdg::XdgPaths::discover()?;
    let settings = settings::load(&paths);
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        None => gui::run(paths)?,
        Some("help") | Some("--help") => print_help(),
        Some("gui") | Some("run") | Some("--gui") => gui::run(paths)?,
        Some("daemon") | Some("service") => gui::run_resident(paths)?,
        Some("update-helper") => {
            let job = args.next().context("update-helper requires a job file")?;
            update::run_helper(&paths, std::path::Path::new(&job))?;
        }
        Some("version") => println!("{}", update::current_version()),
        Some("clipboard-capture-text") => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes)?;
            protonsearch_linux::clipboard::ingest_text(&paths, &bytes)?;
        }
        Some("clipboard-capture-image") => {
            let mut bytes = Vec::new();
            std::io::stdin().read_to_end(&mut bytes)?;
            protonsearch_linux::clipboard::ingest_image(&paths, &bytes)?;
        }
        Some("settings-ui") => gui::run_settings(paths)?,
        Some("doctor") | Some("--print-capabilities") => print_doctor(&paths),
        Some("agent") => run_agent(args.collect::<Vec<_>>().join(" "))?,
        Some("settings") | Some("--settings") => print_json(&settings::catalogue()),
        Some("apps") => {
            let query = args.collect::<Vec<_>>().join(" ");
            let entries = if query.trim().is_empty() {
                desktop::discover_applications(&paths)
            } else {
                desktop::matching_applications(&paths, &query)
            };
            print_json(&entries)
        }
        Some("search") => {
            let query = args.collect::<Vec<_>>().join(" ");
            let options = search::SearchOptions {
                include_hidden: settings.include_hidden,
                max_results: 100,
                max_entries: 50_000,
                max_depth: 32,
                extra_roots: settings.search_roots.iter().map(PathBuf::from).collect(),
                ignored_names: settings.ignored_names.clone(),
            };
            print_json(&search::search_files(&paths, &query, &options))
        }
        Some("search-all") => {
            let query = args.collect::<Vec<_>>().join(" ");
            for item in providers::collect(&paths, &settings, &query) {
                println!("[{}] {} — {}", item.kind, item.title, item.subtitle);
            }
        }
        Some("launch") => {
            let id = args.next().context("launch requires a .desktop filename")?;
            let entry = desktop::discover_applications(&paths)
                .into_iter()
                .find(|entry| entry.id == id)
                .context("desktop entry is not visible or does not exist")?;
            let _child = desktop::launch(&entry)?;
            println!("launched {}", entry.name);
        }
        Some("action") => {
            let action = args
                .next()
                .context("action requires an allowlisted action id")?;
            let mut action_args = Vec::new();
            let mut confirmed = false;
            for arg in args {
                if arg == "--confirm" {
                    confirmed = true;
                } else {
                    action_args.push(arg);
                }
            }
            let result = actions::execute(&paths, &action, &action_args, confirmed)?;
            print_json(&result)
        }
        Some("hyprland") => print_json(&hyprland::discover()),
        Some("open") => {
            let target = args.collect::<Vec<_>>().join(" ");
            let result = system::open_target(&target)?;
            print_json(&result)
        }
        Some(command) => anyhow::bail!("unknown command: {command}; use --help"),
    }
    Ok(())
}

fn print_doctor(paths: &xdg::XdgPaths) {
    let platform = platform::detect();
    let report = serde_json::json!({
        "platform": "linux",
        "distribution": platform.distribution,
        "desktop": platform.desktop,
        "display_server": platform.display_server,
        "package_manager": platform.distribution.package_manager,
        "xdg_config": paths.config,
        "xdg_data": paths.data,
        "xdg_state": paths.state,
        "capabilities": capabilities::detect(),
    });
    print_json(&report);
}

fn print_json(value: &impl serde::Serialize) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => eprintln!("failed to encode result: {error}"),
    }
}

fn run_agent(prompt: String) -> Result<()> {
    let prompt = prompt.trim();
    if prompt.is_empty() {
        anyhow::bail!("agent requires a prompt");
    }
    let command = if system::command_available("hermes") {
        "hermes"
    } else if system::command_available("hermes-agent") {
        "hermes-agent"
    } else {
        anyhow::bail!("Hermes Agent is not installed; install it before using agent prompts");
    };
    let result =
        system::run_with_timeout(command, &["-z", prompt], std::time::Duration::from_secs(90))?;
    if result.timed_out {
        anyhow::bail!("Hermes did not respond within 90 seconds");
    }
    if result.status != Some(0) {
        anyhow::bail!(
            "Hermes exited with status {:?}: {}",
            result.status,
            if result.stderr.is_empty() {
                "no diagnostic output"
            } else {
                &result.stderr
            }
        );
    }
    println!("{}", result.stdout);
    Ok(())
}

fn print_help() {
    println!(
        "ProtonSearch Linux\n\nCommands:\n  gui                   open or toggle the graphical launcher\n  daemon                run the resident background launcher service\n  settings-ui           open Linux settings\n  doctor                 detect Linux providers\n  version                print the installed ProtonSearch version\n  agent <prompt>        send one prompt through the installed Hermes Agent\n  search <query>        search XDG roots\n  search-all <query>    search all Linux providers\n  apps [query]          list or search .desktop applications\n  launch <entry>        launch a visible desktop entry\n  settings              list Linux-only settings\n  action <id> [args]    run an allowlisted provider action\n  hyprland              show read-only Hyprland IPC/config data\n  open <path-or-url>    use the desktop preferred opener\n\nLauncher prefixes include app:, file:, folder:, settings:, browser:, history:, git:, content:, ocr:, clip:, notes:, snippets:, and quicklinks:.\n\nDestructive session actions require --confirm. Packages are never installed automatically."
    );
}

fn refuse_root_and_harden_process() {
    #[cfg(target_os = "linux")]
    unsafe {
        if libc::geteuid() == 0 {
            eprintln!("ProtonSearch Linux refuses to run as root");
            std::process::exit(77);
        }
        libc::umask(0o077);
    }
}
