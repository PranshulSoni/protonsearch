//! Built-in actions surfaced by the launcher (native commands).
//!
//! Everything here maps to a reviewed NativeCommand; destructive commands are
//! flagged so the UI can require confirmation. No shell strings.

use ps_ipc::NativeCommand;

#[derive(Debug, Clone, Copy)]
pub struct CatalogAction {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub keywords: &'static [&'static str],
    pub command: NativeCommand,
    pub destructive: bool,
}

pub const ACTIONS: &[CatalogAction] = &[
    CatalogAction {
        id: "native:lock",
        title: "Lock workstation",
        subtitle: "Windows action",
        keywords: &["lock", "workstation", "screen"],
        command: NativeCommand::LockWorkstation,
        destructive: false,
    },
    CatalogAction {
        id: "native:sleep",
        title: "Sleep",
        subtitle: "Windows action",
        keywords: &["sleep", "suspend", "standby"],
        command: NativeCommand::Sleep,
        destructive: false,
    },
    CatalogAction {
        id: "native:shutdown",
        title: "Shut down",
        subtitle: "Windows action - requires confirmation",
        keywords: &["shutdown", "power", "off"],
        command: NativeCommand::Shutdown,
        destructive: true,
    },
    CatalogAction {
        id: "native:restart",
        title: "Restart",
        subtitle: "Windows action - requires confirmation",
        keywords: &["restart", "reboot", "power"],
        command: NativeCommand::Restart,
        destructive: true,
    },
    CatalogAction {
        id: "native:empty-bin",
        title: "Empty Recycle Bin",
        subtitle: "Windows action - requires confirmation",
        keywords: &["empty", "recycle", "bin", "trash"],
        command: NativeCommand::EmptyRecycleBin,
        destructive: true,
    },
    CatalogAction {
        id: "native:settings",
        title: "Open Windows Settings",
        subtitle: "Windows action",
        keywords: &["settings", "windows", "preferences", "control panel"],
        command: NativeCommand::OpenSettings,
        destructive: false,
    },
    CatalogAction {
        id: "native:task-manager",
        title: "Open Task Manager",
        subtitle: "Windows action",
        keywords: &["task manager", "processes", "performance"],
        command: NativeCommand::OpenTaskManager,
        destructive: false,
    },
    CatalogAction {
        id: "native:terminal",
        title: "Open Terminal",
        subtitle: "Windows action",
        keywords: &["terminal", "command prompt", "cmd", "powershell", "shell"],
        command: NativeCommand::OpenTerminal,
        destructive: false,
    },
    CatalogAction {
        id: "native:pause-index",
        title: "Pause indexing",
        subtitle: "ProtonSearch action",
        keywords: &["pause", "indexing", "index"],
        command: NativeCommand::PauseIndexing,
        destructive: false,
    },
    CatalogAction {
        id: "native:resume-index",
        title: "Resume indexing",
        subtitle: "ProtonSearch action",
        keywords: &["resume", "indexing", "index"],
        command: NativeCommand::ResumeIndexing,
        destructive: false,
    },
];

fn normalize_terms(query: &str) -> Vec<String> {
    query
        .trim()
        .to_lowercase()
        .split_whitespace()
        .map(|term| term.trim_matches(|c: char| c == ':' || c == '"').to_string())
        .filter(|term| !term.is_empty())
        .collect()
}

/// Every query term must appear in the title or keywords (case-insensitive).
pub fn match_actions(query: &str) -> Vec<&'static CatalogAction> {
    let terms = normalize_terms(query);
    if terms.is_empty() {
        return Vec::new();
    }
    ACTIONS
        .iter()
        .filter(|action| {
            let haystack = format!("{} {}", action.title.to_lowercase(), action.keywords.join(" "));
            terms.iter().all(|term| haystack.contains(term.as_str()))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_single_and_multi_term() {
        assert_eq!(match_actions("lock").len(), 1);
        assert!(match_actions("empty recycle bin")
            .iter()
            .any(|action| action.id == "native:empty-bin"));
        assert!(match_actions("task manager")
            .iter()
            .any(|action| action.id == "native:task-manager"));
        assert!(match_actions("zzzzz").is_empty());
    }

    #[test]
    fn destructive_commands_are_flagged() {
        for action in ACTIONS {
            match action.command {
                NativeCommand::Shutdown
                | NativeCommand::Restart
                | NativeCommand::EmptyRecycleBin => assert!(action.destructive),
                _ => assert!(!action.destructive),
            }
        }
    }
}
