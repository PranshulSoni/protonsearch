//! Built-in actions surfaced by the launcher (native commands + natural-language
//! intents such as "restart my pc" or "turn off the computer").
//!
//! Matching is deterministic and local: lowercase, punctuation-insensitive,
//! filler-word tolerant, alias-based for natural phrasing, and with a bounded
//! fuzzy fallback for typos. There is no LLM in this path.

use ps_ipc::NativeCommand;

#[derive(Debug, Clone, Copy)]
pub struct CatalogAction {
    pub id: &'static str,
    pub title: &'static str,
    pub subtitle: &'static str,
    pub keywords: &'static [&'static str],
    /// Natural-language phrasings users actually type.
    pub aliases: &'static [&'static str],
    pub command: NativeCommand,
    pub destructive: bool,
}

pub const ACTIONS: &[CatalogAction] = &[
    CatalogAction {
        id: "native:lock",
        title: "Lock workstation",
        subtitle: "Windows action",
        keywords: &["lock", "workstation", "screen"],
        aliases: &[
            "lock pc", "lock my pc", "lock computer", "lock my computer",
            "lock screen", "lock the screen", "lock this pc", "lock windows",
        ],
        command: NativeCommand::LockWorkstation,
        destructive: false,
    },
    CatalogAction {
        id: "native:sleep",
        title: "Sleep",
        subtitle: "Windows action",
        keywords: &["sleep", "suspend", "standby"],
        aliases: &[
            "sleep pc", "sleep my pc", "put pc to sleep", "put my pc to sleep",
            "put the computer to sleep", "sleep computer", "suspend",
        ],
        command: NativeCommand::Sleep,
        destructive: false,
    },
    CatalogAction {
        id: "native:shutdown",
        title: "Shut down",
        subtitle: "Windows action - requires confirmation",
        keywords: &["shutdown", "power", "off"],
        aliases: &[
            "shut down", "shutdown", "shut down pc", "shut down my pc",
            "shut down computer", "shut down my computer", "shut down this pc",
            "turn off pc", "turn off my pc", "turn off computer",
            "turn off my computer", "turn off this pc", "turn my pc off",
            "turn the pc off", "power off", "power down", "power off pc",
        ],
        command: NativeCommand::Shutdown,
        destructive: true,
    },
    CatalogAction {
        id: "native:restart",
        title: "Restart",
        subtitle: "Windows action - requires confirmation",
        keywords: &["restart", "reboot", "power"],
        aliases: &[
            "restart", "restart pc", "restart my pc", "restart computer",
            "restart my computer", "restart this pc", "reboot", "reboot pc",
            "reboot my pc", "reboot computer", "reboot my computer",
            "restart the computer",
        ],
        command: NativeCommand::Restart,
        destructive: true,
    },
    CatalogAction {
        id: "native:empty-bin",
        title: "Empty Recycle Bin",
        subtitle: "Windows action - requires confirmation",
        keywords: &["empty", "recycle", "bin", "trash", "cleanup"],
        aliases: &[
            "empty recycle bin", "empty the recycle bin", "empty bin",
            "empty trash", "empty trash bin", "empty the trash",
            "clear recycle bin", "clear the recycle bin", "clear trash",
        ],
        command: NativeCommand::EmptyRecycleBin,
        destructive: true,
    },
    CatalogAction {
        id: "native:settings",
        title: "Open Windows Settings",
        subtitle: "Windows action",
        keywords: &["settings", "windows", "preferences", "control panel"],
        aliases: &[
            "open settings", "open windows settings", "windows settings",
            "settings app", "control panel", "open control panel",
        ],
        command: NativeCommand::OpenSettings,
        destructive: false,
    },
    CatalogAction {
        id: "native:task-manager",
        title: "Open Task Manager",
        subtitle: "Windows action",
        keywords: &["task manager", "processes", "performance", "process"],
        aliases: &[
            "task manager", "open task manager", "open the task manager",
            "running processes", "process explorer",
        ],
        command: NativeCommand::OpenTaskManager,
        destructive: false,
    },
    CatalogAction {
        id: "native:terminal",
        title: "Open Terminal",
        subtitle: "Windows action",
        keywords: &["terminal", "command prompt", "cmd", "powershell", "shell"],
        aliases: &[
            "terminal", "open terminal", "open the terminal", "command prompt",
            "open command prompt", "cmd", "powershell", "open powershell", "shell",
        ],
        command: NativeCommand::OpenTerminal,
        destructive: false,
    },
    CatalogAction {
        id: "native:pause-index",
        title: "Pause indexing",
        subtitle: "ProtonSearch action",
        keywords: &["pause", "indexing", "index", "stop"],
        aliases: &["pause indexing", "pause the index", "stop indexing", "pause index"],
        command: NativeCommand::PauseIndexing,
        destructive: false,
    },
    CatalogAction {
        id: "native:resume-index",
        title: "Resume indexing",
        subtitle: "ProtonSearch action",
        keywords: &["resume", "indexing", "index", "start"],
        aliases: &[
            "resume indexing", "resume the index", "start indexing",
            "start indexing again", "resume index",
        ],
        command: NativeCommand::ResumeIndexing,
        destructive: false,
    },
];

/// Filler words that never carry intent in a short command query.
const FILLERS: &[&str] = &[
    "my", "the", "a", "an", "please", "now", "immediately", "for", "me", "i",
    "want", "to", "can", "you", "this", "that", "it", "computer", "machine",
    "pc", "system", "quickly", "right", "away",
];

struct ActionIndex {
    /// Normalised token set for coverage matching.
    tokens: Vec<String>,
    /// Compact alias/keyword phrases for exact/prefix matching.
    phrases: Vec<String>,
}

fn normalise(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_alphanumeric() {
            out.extend(ch.to_lowercase());
        } else {
            out.push(' ');
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn compact(input: &str) -> String {
    input.chars().filter(|ch| ch.is_alphanumeric()).collect()
}

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for (i, ca) in a.iter().enumerate() {
        cur[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let cost = if ca == cb { 0 } else { 1 };
            cur[j + 1] = (prev[j + 1] + 1).min(cur[j] + 1).min(prev[j] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

fn index_of(action: &'static CatalogAction) -> ActionIndex {
    let mut phrases: Vec<String> = Vec::new();
    for keyword in action.keywords {
        phrases.push(normalise(keyword));
    }
    for alias in action.aliases {
        phrases.push(normalise(alias));
    }
    phrases.push(normalise(action.title));
    let compacted: Vec<String> = phrases.iter().map(|phrase| compact(phrase)).collect();
    let mut tokens: Vec<String> = Vec::new();
    for phrase in &phrases {
        for token in phrase.split_whitespace() {
            if !tokens.iter().any(|existing| existing == token) {
                tokens.push(token.to_string());
            }
        }
    }
    ActionIndex { tokens, phrases: compacted }
}

/// Query tokens after filler removal, normalised and deduplicated.
fn query_tokens(query: &str) -> Vec<String> {
    let normalised = normalise(query);
    let mut tokens = Vec::new();
    for token in normalised.split_whitespace() {
        if FILLERS.contains(&token) {
            continue;
        }
        if !tokens.iter().any(|existing| existing == token) {
            tokens.push(token.to_string());
        }
    }
    tokens
}

fn score(index: &ActionIndex, tokens: &[String], query_compact: &str) -> Option<u32> {
    if tokens.is_empty() {
        return None;
    }
    // 1. Exact phrase match (word-order and punctuation insensitive).
    if index.phrases.iter().any(|phrase| phrase == query_compact) {
        return Some(1000);
    }
    // 2. Prefix phrase match for phrases at least 4 chars (e.g. "restar" -> restart).
    if query_compact.len() >= 4
        && index
            .phrases
            .iter()
            .any(|phrase| phrase.starts_with(query_compact))
    {
        return Some(900);
    }
    // 3. Every query token covered by an action token.
    if tokens
        .iter()
        .all(|token| index.tokens.iter().any(|candidate| candidate == token))
    {
        return Some(820);
    }
    // 4. Every query token is a prefix of (or extends) an action token.
    if tokens.iter().all(|token| {
        token.len() >= 3
            && index.tokens.iter().any(|candidate| {
                candidate.starts_with(token.as_str()) || token.starts_with(candidate.as_str())
            })
    }) {
        return Some(700);
    }
    // 5. Bounded typo tolerance: each token within edit distance 1 of a token.
    if tokens.iter().all(|token| {
        token.len() >= 5
            && index
                .tokens
                .iter()
                .any(|candidate| candidate.len() >= 5 && levenshtein(token, candidate) <= 1)
    }) {
        return Some(500);
    }
    None
}

/// Match the built-in action catalog against a natural-language query.
///
/// Examples: "restart my pc", "shut down the computer", "turn off my pc",
/// "lock screen", "empty the recycle bin", "open task manager please".
/// Results are ordered best-match first. Deterministic; no network/LLM.
pub fn match_actions(query: &str) -> Vec<&'static CatalogAction> {
    let tokens = query_tokens(query);
    if tokens.is_empty() {
        return Vec::new();
    }
    let query_compact: String = tokens.join("");
    if query_compact.is_empty() {
        return Vec::new();
    }
    let mut scored: Vec<(&'static CatalogAction, u32)> = ACTIONS
        .iter()
        .filter_map(|action| {
            let index = index_of(action);
            score(&index, &tokens, &query_compact).map(|score| (action, score))
        })
        .collect();
    scored.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.id.cmp(b.0.id)));
    scored.into_iter().map(|(action, _)| action).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(query: &str) -> Vec<&'static str> {
        match_actions(query).iter().map(|action| action.id).collect()
    }

    #[test]
    fn natural_language_restart_and_shutdown() {
        assert_eq!(ids("restart my PC"), vec!["native:restart"]);
        assert_eq!(ids("shut down my PC"), vec!["native:shutdown"]);
        assert_eq!(ids("Shut Down the computer please"), vec!["native:shutdown"]);
        assert_eq!(ids("turn off my computer"), vec!["native:shutdown"]);
        assert_eq!(ids("power off pc"), vec!["native:shutdown"]);
        assert_eq!(ids("reboot"), vec!["native:restart"]);
        assert_eq!(ids("restart computer now"), vec!["native:restart"]);
    }

    #[test]
    fn natural_language_other_actions() {
        assert_eq!(ids("lock my screen"), vec!["native:lock"]);
        assert_eq!(ids("lock pc"), vec!["native:lock"]);
        assert_eq!(ids("put my pc to sleep"), vec!["native:sleep"]);
        assert_eq!(ids("empty the recycle bin"), vec!["native:empty-bin"]);
        assert_eq!(ids("empty trash"), vec!["native:empty-bin"]);
        assert_eq!(ids("open task manager please"), vec!["native:task-manager"]);
        assert_eq!(ids("open windows settings"), vec!["native:settings"]);
        assert_eq!(ids("open terminal"), vec!["native:terminal"]);
        assert_eq!(ids("pause indexing"), vec!["native:pause-index"]);
    }

    #[test]
    fn shutdown_and_restart_do_not_cross_match() {
        assert!(!ids("restart my pc").contains(&"native:shutdown"));
        assert!(!ids("shut down my pc").contains(&"native:restart"));
    }

    #[test]
    fn unrelated_queries_do_not_match() {
        assert!(ids("zzzzz").is_empty());
        assert!(ids("restart driver").is_empty());
        assert!(ids("quarterly report").is_empty());
        assert!(ids("my pc").is_empty());
    }

    #[test]
    fn typo_tolerance_is_bounded() {
        assert_eq!(ids("restar my pc"), vec!["native:restart"]);
        assert_eq!(ids("shutdwn my pc"), vec!["native:shutdown"]);
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