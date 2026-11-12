//! Application name matching for the Apps and All filters.
//!
//! An app matches on its display name, on aliases (the executable it launches,
//! e.g. "chrome" for Google Chrome, plus a few well-known short names such as
//! "vscode"), on word prefixes and initials ("vis stu", "vsc"), by substring and,
//! for longer queries, with one typo. Names and queries are case- and
//! accent-folded so "resume" finds "Résumé" in either Unicode form.
//!
//! App lists are small (hundreds), so each app is prepared once per app list
//! and matching is plain loops over chars.

/// How an app matched, strongest first, in the order the Apps filter ranks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AppMatch {
    ExactName,
    ExactAlias,
    Prefix,
    ExactToken,
    WordPrefix,
    Substring,
    StrongFuzzy,
    WeakFuzzy,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AppHit {
    pub kind: AppMatch,
    /// 0..1 closeness inside the class (shorter names, tighter fuzzy spans).
    pub quality: f32,
    /// UTF-16 highlight ranges in the display name.
    pub ranges: Vec<(usize, usize)>,
}

/// One app name folded once for matching.
#[derive(Debug, Clone)]
pub struct PreparedApp {
    /// Folded (lower-case, accent-free) chars of the display name.
    chars: Vec<char>,
    /// UTF-16 start/end in the original name for every folded char.
    spans: Vec<(usize, usize)>,
    /// Word ranges (char indices into `chars`), also split at case and
    /// letter/digit changes ("PowerShell" -> power, shell).
    words: Vec<(usize, usize)>,
    /// Separator-delimited tokens ("PowerShell" stays one token).
    tokens: Vec<(usize, usize)>,
    /// The launched executable's name, when it identifies the app.
    exe: Option<String>,
}

/// Executable names shared by unrelated apps (launcher stubs, runtimes,
/// installers): never treated as an alias.
const GENERIC_EXES: &[&str] = &[
    "update", "updater", "launcher", "launch", "setup", "install", "installer", "uninstall", "unins000",
    "app", "application", "start", "run", "main", "electron", "java", "javaw", "javaws", "python",
    "pythonw", "py", "node", "rundll32", "msiexec", "wscript", "cscript", "mshta", "conhost", "dllhost",
    "helper", "client", "bootstrapper", "squirrel", "stub",
];

/// Well-known short names that are not the executable's name. Each alias maps
/// to folded display names; an app named exactly so (or "<name> - ...") counts.
const KNOWN_ALIASES: &[(&str, &[&str])] = &[
    ("vscode", &["visual studio code"]),
    ("vs code", &["visual studio code"]),
    ("code", &["visual studio code"]),
    ("vs", &["visual studio"]),
    ("word", &["word", "microsoft word"]),
    ("excel", &["excel", "microsoft excel"]),
    ("powerpoint", &["powerpoint", "microsoft powerpoint"]),
    ("ppt", &["powerpoint", "microsoft powerpoint"]),
    ("outlook", &["outlook", "microsoft outlook", "outlook (new)"]),
    ("onenote", &["onenote", "microsoft onenote"]),
    ("teams", &["microsoft teams", "microsoft teams (work or school)", "teams"]),
    ("edge", &["microsoft edge"]),
    ("chrome", &["google chrome"]),
    ("firefox", &["firefox", "mozilla firefox"]),
    ("calc", &["calculator"]),
    ("cmd", &["command prompt"]),
    ("terminal", &["terminal", "windows terminal"]),
    ("wt", &["terminal", "windows terminal"]),
    ("powershell", &["windows powershell", "powershell"]),
    ("pwsh", &["powershell", "powershell 7 (x64)"]),
    ("ps", &["windows powershell", "powershell"]),
    ("explorer", &["file explorer"]),
    ("files", &["file explorer"]),
    ("task manager", &["task manager"]),
    ("taskmgr", &["task manager"]),
    ("regedit", &["registry editor"]),
    ("control", &["control panel"]),
    ("snip", &["snipping tool"]),
    ("store", &["microsoft store"]),
    ("settings", &["settings"]),
    ("photos", &["photos", "microsoft photos"]),
    ("paint", &["paint"]),
];

/// Lower-case one char and strip Latin accents; `None` for combining marks so
/// decomposed ("e" + U+0301) and precomposed ("é") names fold the same.
fn fold_char(c: char) -> Option<char> {
    if ('\u{0300}'..='\u{036f}').contains(&c) {
        return None;
    }
    let lower = c.to_lowercase().next().unwrap_or(c);
    if lower.is_ascii() {
        return Some(lower);
    }
    const FROM: &str = "àáâãäåāăąçćĉċčďđèéêëēĕėęěĝğġģĥħìíîïĩīĭįıĵķĺļľŀłñńņňòóôõöøōŏőŕŗřśŝşšţťŧùúûüũūŭůűųŵýÿŷźżžß";
    const TO: &str = "aaaaaaaaacccccddeeeeeeeeegggghhiiiiiiiiijklllllnnnnooooooooorrrsssstttuuuuuuuuuuwyyyzzzs";
    Some(
        FROM.chars()
            .position(|from| from == lower)
            .and_then(|i| TO.chars().nth(i))
            .unwrap_or(lower),
    )
}

/// Folded query text: lower-case, accent-free, whitespace collapsed.
pub fn fold_query(query: &str) -> String {
    let folded: String = query.chars().filter_map(fold_char).collect();
    folded.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn is_separator(c: char) -> bool {
    !c.is_alphanumeric()
}

/// The executable a classic app launches, when its name identifies the app.
fn exe_alias(target: &str) -> Option<String> {
    let file = target.rsplit(['\\', '/']).next()?;
    let (stem, ext) = file.rsplit_once('.')?;
    if !ext.eq_ignore_ascii_case("exe") {
        return None;
    }
    let stem = fold_query(stem);
    if stem.chars().count() < 2 || GENERIC_EXES.contains(&stem.as_str()) {
        return None;
    }
    Some(stem)
}

impl PreparedApp {
    pub fn new(name: &str, target: &str) -> Self {
        let mut chars = Vec::new();
        let mut spans: Vec<(usize, usize)> = Vec::new();
        let mut upper = Vec::new();
        let mut utf16 = 0usize;
        for c in name.chars() {
            let width = c.len_utf16();
            match fold_char(c) {
                Some(folded) => {
                    chars.push(folded);
                    spans.push((utf16, utf16 + width));
                    upper.push(c.is_uppercase());
                }
                // A combining mark belongs to the previous char's highlight.
                None => {
                    if let Some(last) = spans.last_mut() {
                        last.1 = utf16 + width;
                    }
                }
            }
            utf16 += width;
        }
        let mut words = Vec::new();
        let mut start: Option<usize> = None;
        for i in 0..chars.len() {
            if is_separator(chars[i]) {
                if let Some(s) = start.take() {
                    words.push((s, i));
                }
                continue;
            }
            let boundary = i > 0
                && !is_separator(chars[i - 1])
                && ((upper[i] && !upper[i - 1])
                    || (chars[i].is_numeric() != chars[i - 1].is_numeric()));
            match start {
                Some(s) if boundary => {
                    words.push((s, i));
                    start = Some(i);
                }
                None => start = Some(i),
                Some(_) => {}
            }
        }
        if let Some(s) = start {
            words.push((s, chars.len()));
        }
        let mut tokens: Vec<(usize, usize)> = Vec::new();
        for &(s, e) in &words {
            match tokens.last_mut() {
                Some(last) if last.1 == s => last.1 = e,
                _ => tokens.push((s, e)),
            }
        }
        Self {
            chars,
            spans,
            words,
            tokens,
            exe: exe_alias(target),
        }
    }

    fn word(&self, i: usize) -> &[char] {
        let (s, e) = self.words[i];
        &self.chars[s..e]
    }

    /// UTF-16 range covering folded chars start..end.
    fn range(&self, start: usize, end: usize) -> (usize, usize) {
        (self.spans[start].0, self.spans[end - 1].1)
    }

    /// The name as words joined by single spaces ("Notepad++" -> "notepad").
    fn words_text(&self) -> String {
        let words: Vec<String> = (0..self.words.len()).map(|i| self.word(i).iter().collect()).collect();
        words.join(" ")
    }

    fn has_alias(&self, query: &str) -> bool {
        if self.exe.as_deref() == Some(query) {
            return true;
        }
        let name: String = self.chars.iter().collect();
        KNOWN_ALIASES.iter().any(|(alias, names)| {
            *alias == query
                && names
                    .iter()
                    .any(|known| name == *known || name.starts_with(&format!("{known} - ")))
        })
    }
}

fn find(hay: &[char], needle: &[char]) -> Option<usize> {
    if needle.is_empty() || needle.len() > hay.len() {
        return None;
    }
    (0..=hay.len() - needle.len()).find(|&i| hay[i..i + needle.len()] == *needle)
}

/// Can `query` be written as prefixes of two or more of the words, in order
/// ("vsc", "vscode", "gchrome")? Returns the chars used per word.
fn initials(app: &PreparedApp, query: &[char], word: usize, used: &mut Vec<(usize, usize)>) -> bool {
    if query.is_empty() {
        return used.len() >= 2;
    }
    for w in word..app.words.len() {
        let text = app.word(w);
        let mut common = 0usize;
        while common < text.len() && common < query.len() && text[common] == query[common] {
            common += 1;
        }
        for take in (1..=common).rev() {
            used.push((w, take));
            if initials(app, &query[take..], w + 1, used) {
                return true;
            }
            used.pop();
        }
    }
    false
}

/// Damerau-Levenshtein distance of at most one.
fn within_one_edit(a: &[char], b: &[char]) -> bool {
    let (long, short) = if a.len() >= b.len() { (a, b) } else { (b, a) };
    match long.len() - short.len() {
        0 => {
            let diff: Vec<usize> = (0..long.len()).filter(|&i| long[i] != short[i]).collect();
            match diff.len() {
                0 | 1 => true,
                2 => diff[1] == diff[0] + 1 && long[diff[0]] == short[diff[1]] && long[diff[1]] == short[diff[0]],
                _ => false,
            }
        }
        1 => {
            let mut skipped = false;
            let (mut i, mut j) = (0, 0);
            while i < long.len() && j < short.len() {
                if long[i] == short[j] {
                    i += 1;
                    j += 1;
                } else if skipped {
                    return false;
                } else {
                    skipped = true;
                    i += 1;
                }
            }
            true
        }
        _ => false,
    }
}

/// Greedy subsequence over the name; quality from span compactness.
fn subsequence(app: &PreparedApp, query: &[char]) -> Option<(f32, Vec<usize>)> {
    let mut positions = Vec::with_capacity(query.len());
    let mut qi = 0usize;
    for (i, c) in app.chars.iter().enumerate() {
        if qi < query.len() && *c == query[qi] {
            positions.push(i);
            qi += 1;
        }
    }
    if qi < query.len() {
        return None;
    }
    let span = positions.last()? - positions.first()? + 1;
    if span > query.len() * 3 {
        return None;
    }
    Some((query.len() as f32 / span as f32, positions))
}

fn merge_ranges(app: &PreparedApp, positions: &[usize]) -> Vec<(usize, usize)> {
    let mut out: Vec<(usize, usize)> = Vec::new();
    for &p in positions {
        let (s, e) = app.range(p, p + 1);
        match out.last_mut() {
            Some(last) if last.1 == s => last.1 = e,
            _ => out.push((s, e)),
        }
    }
    out
}

/// Match a folded query (see [`fold_query`]) against one prepared app.
pub fn match_app(app: &PreparedApp, query: &str) -> Option<AppHit> {
    let q: Vec<char> = query.chars().collect();
    if q.is_empty() || app.chars.is_empty() {
        return None;
    }
    let terms: Vec<Vec<char>> = query.split(' ').map(|t| t.chars().collect()).collect();
    let compact: Vec<char> = q.iter().copied().filter(|c| *c != ' ').collect();
    let n = app.chars.len();
    let closeness = |matched: usize| (matched as f32 / n as f32).clamp(0.0, 1.0);
    let hit = |kind, quality, ranges| Some(AppHit { kind, quality, ranges });

    // 1. exact display name (punctuation-insensitive: "Paint 3D" = "paint 3d")
    if app.chars == q || app.words_text() == query {
        return hit(AppMatch::ExactName, 1.0, vec![app.range(0, n)]);
    }
    // 2. exact alias ("chrome" -> chrome.exe, "vscode")
    let compact_text: String = compact.iter().collect();
    if app.has_alias(query) || app.has_alias(&compact_text) {
        return hit(AppMatch::ExactAlias, 1.0, Vec::new());
    }
    // 3. name prefix
    if app.chars.starts_with(&q) {
        return hit(AppMatch::Prefix, closeness(q.len()), vec![app.range(0, q.len())]);
    }
    // 4. exact token: the query is whole words of the name
    let word_count = app.words.len();
    for parts in [&app.tokens, &app.words] {
        if terms.len() > parts.len() {
            continue;
        }
        for k in 0..=parts.len() - terms.len() {
            let whole = terms.iter().enumerate().all(|(j, term)| {
                let (s, e) = parts[k + j];
                app.chars[s..e] == *term.as_slice()
            });
            if whole {
                let start = parts[k].0;
                let end = parts[k + terms.len() - 1].1;
                return hit(AppMatch::ExactToken, closeness(end - start), vec![app.range(start, end)]);
            }
        }
    }
    // 5. ordered word prefixes: "stu" -> Studio, "vis stu", initials "vsc"
    let mut used = Vec::new();
    let mut next_word = 0usize;
    let mut ranges = Vec::new();
    let mut ordered = true;
    for term in &terms {
        let found = (next_word..word_count).find(|&w| app.word(w).starts_with(term));
        match found {
            Some(w) => {
                let start = app.words[w].0;
                ranges.push(app.range(start, start + term.len()));
                next_word = w + 1;
            }
            None => {
                ordered = false;
                break;
            }
        }
    }
    if ordered {
        return hit(AppMatch::WordPrefix, closeness(compact.len()), ranges);
    }
    if compact.len() >= 2 && initials(app, &compact, 0, &mut used) {
        let ranges = used
            .iter()
            .map(|&(w, take)| app.range(app.words[w].0, app.words[w].0 + take))
            .collect();
        return hit(AppMatch::WordPrefix, closeness(compact.len()), ranges);
    }
    // 6. substring (two or more chars)
    if q.len() >= 2 {
        if let Some(start) = find(&app.chars, &q) {
            return hit(AppMatch::Substring, closeness(q.len()), vec![app.range(start, start + q.len())]);
        }
    }
    // 7. one typo (4+ chars): against the whole name or the executable alias
    // it names the app itself; against one word of a longer name it is weaker.
    if compact.len() >= 4 {
        let name_compact: Vec<char> = app.chars.iter().copied().filter(|c| !is_separator(*c)).collect();
        let exe: Vec<char> = app.exe.as_deref().unwrap_or("").chars().collect();
        if within_one_edit(&name_compact, &compact) || (exe.len() >= 4 && within_one_edit(&exe, &compact)) {
            return hit(AppMatch::StrongFuzzy, 0.75, Vec::new());
        }
        if terms.len() == 1 {
            let word = (0..word_count).find(|&w| app.word(w).len() >= 4 && within_one_edit(app.word(w), &compact));
            if let Some(w) = word {
                let (s, e) = app.words[w];
                return hit(AppMatch::StrongFuzzy, 0.5 * closeness(e - s), vec![app.range(s, e)]);
            }
        }
    }
    // 8. scattered subsequence (3+ chars)
    if compact.len() >= 3 {
        if let Some((quality, positions)) = subsequence(app, &compact) {
            let kind = if quality >= 0.5 { AppMatch::StrongFuzzy } else { AppMatch::WeakFuzzy };
            return hit(kind, quality, merge_ranges(app, &positions));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind(name: &str, target: &str, query: &str) -> Option<AppMatch> {
        match_app(&PreparedApp::new(name, target), &fold_query(query)).map(|hit| hit.kind)
    }

    #[test]
    fn accent_table_is_aligned() {
        assert_eq!(fold_query("àáâãäåāăąçćĉċčďđèéêëēĕėęěĝğġģĥħìíîïĩīĭįıĵķĺļľŀłñńņňòóôõöøōŏőŕŗřśŝşšţťŧùúûüũūŭůűųŵýÿŷźżžß").chars().count(), 88);
        assert_eq!(fold_query("Ñandú Ørsted Łódź"), "nandu orsted lodz");
        assert_eq!(fold_query("Résumé ÇA"), "resume ca");
        // Decomposed form folds the same as precomposed.
        assert_eq!(fold_query("Re\u{301}sume\u{301}"), "resume");
    }

    #[test]
    fn match_classes_follow_the_apps_filter_order() {
        let chrome = r"C:\Program Files\Google\Chrome\Application\chrome.exe";
        assert_eq!(kind("Google Chrome", chrome, "google chrome"), Some(AppMatch::ExactName));
        assert_eq!(kind("Google Chrome", chrome, "chrome"), Some(AppMatch::ExactAlias));
        assert_eq!(kind("Chrome Remote Desktop", r"C:\x\remoting_start_host.exe", "chrome"), Some(AppMatch::Prefix));
        assert_eq!(kind("Google Chrome", chrome, "goo"), Some(AppMatch::Prefix));
        assert_eq!(kind("Windows PowerShell ISE", r"C:\x\powershell_ise.exe", "powershell"), Some(AppMatch::ExactToken));
        assert_eq!(kind("Visual Studio Code", r"C:\x\Code.exe", "stu"), Some(AppMatch::WordPrefix));
        assert_eq!(kind("Visual Studio Code", r"C:\x\Code.exe", "vis cod"), Some(AppMatch::WordPrefix));
        assert_eq!(kind("Visual Studio Code", r"C:\x\Code.exe", "vsc"), Some(AppMatch::WordPrefix));
        assert_eq!(kind("Notepad++", r"C:\x\notepad++.exe", "pad"), Some(AppMatch::Substring));
        assert_eq!(kind("Spotify", r"C:\x\Spotify.exe", "spotfy"), Some(AppMatch::StrongFuzzy));
        assert_eq!(kind("Google Chrome", chrome, "crome"), Some(AppMatch::StrongFuzzy));
        assert_eq!(kind("Spotify", r"C:\x\Spotify.exe", "zz"), None);
        // Letters scattered across a long name are not a match.
        assert_eq!(kind("Epic Games Launcher", r"C:\x\EpicGamesLauncher.exe", "calc"), None);
    }

    #[test]
    fn aliases_cover_executables_and_known_names() {
        assert_eq!(kind("Visual Studio Code", r"C:\x\Code.exe", "vscode"), Some(AppMatch::ExactAlias));
        assert_eq!(kind("Visual Studio Code", r"C:\x\Code.exe", "vs code"), Some(AppMatch::ExactAlias));
        assert_eq!(kind("Microsoft Edge", r"C:\x\msedge.exe", "msedge"), Some(AppMatch::ExactAlias));
        assert_eq!(kind("Calculator", "shell:AppsFolder\\Microsoft.WindowsCalculator!App", "calc"), Some(AppMatch::ExactAlias));
        // Launcher stubs are not aliases: "update" must not mean Discord.
        assert_ne!(kind("Discord", r"C:\x\Update.exe", "update"), Some(AppMatch::ExactAlias));
    }

    #[test]
    fn one_letter_queries_stay_on_word_starts() {
        assert_eq!(kind("Spotify", r"C:\x\Spotify.exe", "s"), Some(AppMatch::Prefix));
        assert_eq!(kind("Microsoft Teams", r"C:\x\ms-teams.exe", "t"), Some(AppMatch::WordPrefix));
        assert_eq!(kind("Notepad", r"C:\x\notepad.exe", "e"), None);
    }

    #[test]
    fn highlight_ranges_are_utf16_in_the_original_name() {
        let app = PreparedApp::new("Résumé Builder", r"C:\x\rb.exe");
        let hit = match_app(&app, &fold_query("resume")).unwrap();
        assert_eq!(hit.kind, AppMatch::Prefix);
        assert_eq!(hit.ranges, vec![(0, 6)]);
        let decomposed = PreparedApp::new("Re\u{301}sume\u{301} Builder", r"C:\x\rb.exe");
        let hit = match_app(&decomposed, &fold_query("resume")).unwrap();
        assert_eq!(hit.ranges, vec![(0, 8)]);
    }
}
