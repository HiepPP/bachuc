//! The slash command menu model: which commands exist, how a typed query filters them, and which
//! client command a sent text runs. The client commands mirror
//! `packages/app/src/client-slash-commands/index.ts`.

use bachuc_client::SlashCommand;

/// A command the menu lists.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashItem {
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    /// The client runs it; it never reaches the agent.
    pub client: Option<ClientCommand>,
    /// A plugin runs it with its arguments; it never reaches the agent.
    pub plugin: bool,
    /// The daemon lists it with kind "skill"; only skills show after other text.
    pub skill: bool,
}

/// A command the daemon lists, as `AgentSlashCommand` in the Electron app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DaemonCommand {
    pub name: String,
    pub description: String,
    /// `kind == "skill"`.
    pub skill: bool,
}

impl From<&SlashCommand> for DaemonCommand {
    fn from(command: &SlashCommand) -> Self {
        DaemonCommand {
            name: command.name.clone(),
            description: command.description.clone(),
            skill: command.skill,
        }
    }
}

/// What a client command does when sent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClientCommand {
    /// `/exit`: archive the agent.
    Exit,
    /// `/clear`: archive the agent and ask the owner for a fresh draft.
    Clear,
}

/// The client commands, in menu order.
pub fn client_items() -> Vec<SlashItem> {
    let item = |name: &str, aliases: &[&str], description: &str, command| SlashItem {
        name: name.to_string(),
        description: description.to_string(),
        aliases: aliases.iter().map(|alias| alias.to_string()).collect(),
        client: Some(command),
        plugin: false,
        skill: false,
    };
    vec![
        item(
            "exit",
            &["quit", "q"],
            "Archive the current agent",
            ClientCommand::Exit,
        ),
        item(
            "clear",
            &["new"],
            "Archive this agent and start a fresh draft",
            ClientCommand::Clear,
        ),
    ]
}

/// The client commands first, then the daemon commands, as `mergeSlashCommandSources` does. A
/// client command name or alias hides a daemon command of that name.
#[cfg(test)]
pub fn menu_items(daemon: &[DaemonCommand]) -> Vec<SlashItem> {
    merge_items(daemon, &[])
}

/// `mergeSlashCommandSources`: the built-in commands, then the plugin commands whose name no
/// built-in name or alias claims, then the daemon commands no earlier entry claims. `plugins`
/// are `(name, description)` in registration order.
pub fn merge_items(daemon: &[DaemonCommand], plugins: &[(String, String)]) -> Vec<SlashItem> {
    let mut items = client_items();
    let claimed = |items: &[SlashItem], name: &str| {
        items
            .iter()
            .any(|item| item.name == name || item.aliases.iter().any(|alias| alias == name))
    };
    for (name, description) in plugins {
        if claimed(&items, name) {
            log::warn!("plugin slash command /{name} is hidden by a built-in command");
            continue;
        }
        items.push(SlashItem {
            name: name.clone(),
            description: description.clone(),
            aliases: Vec::new(),
            client: None,
            plugin: true,
            skill: false,
        });
    }
    let daemon: Vec<SlashItem> = daemon
        .iter()
        .filter(|command| !claimed(&items, &command.name))
        .map(|command| SlashItem {
            name: command.name.clone(),
            description: command.description.clone(),
            aliases: Vec::new(),
            client: None,
            plugin: false,
            skill: command.skill,
        })
        .collect();
    items.extend(daemon);
    items
}

/// `resolvePluginClientSlashCommand`: with no attachment, `/name args` whose name is one of
/// `names` gives that name and the trimmed arguments.
pub fn resolve_plugin_command<'a>(
    text: &str,
    attachments: usize,
    names: &[&'a str],
) -> Option<(&'a str, String)> {
    if attachments > 0 {
        return None;
    }
    let rest = text.trim().strip_prefix('/')?;
    let (name, args) = match rest.find(char::is_whitespace) {
        Some(at) => (&rest[..at], rest[at..].trim()),
        None => (rest, ""),
    };
    if name.is_empty() {
        return None;
    }
    let name = names.iter().find(|candidate| **candidate == name)?;
    Some((name, args.to_string()))
}

/// A `/name` the caret is in: `findActiveSlashCommand`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlashRange {
    /// Byte offset of the `/`.
    pub start: usize,
    /// Byte offset of the caret.
    pub end: usize,
    pub query: String,
    /// The `/` follows other text on its line, so only skills may be picked.
    pub inline: bool,
}

/// `findActiveSlashCommand`: the nearest `/` before the caret that starts the text or follows
/// whitespace, with no whitespace, `/`, or quote between it and the caret.
pub fn find_active_slash(text: &str, cursor: usize) -> Option<SlashRange> {
    let mut end = cursor.min(text.len());
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let before = &text[..end];
    let mut search = before;
    while let Some(start) = search.rfind('/') {
        search = &before[..start];
        if before[..start]
            .chars()
            .next_back()
            .is_some_and(|previous| !previous.is_whitespace())
        {
            continue;
        }
        let query = &before[start + 1..];
        if query
            .chars()
            .any(|c| c == '/' || c == '"' || c == '\'' || c.is_whitespace())
        {
            continue;
        }
        return Some(SlashRange {
            start,
            end,
            query: query.to_string(),
            inline: start != 0,
        });
    }
    None
}

/// The rows the menu shows for a caret in `range`, best first. A draft lists no built-in
/// command, and a `/` after text lists skills only, as `buildCommandAutocompleteOptions` does.
pub fn menu_matches(
    daemon: &[DaemonCommand],
    plugins: &[(String, String)],
    range: &SlashRange,
    draft: bool,
) -> Vec<SlashItem> {
    let mut items = merge_items(daemon, plugins);
    if draft {
        items.retain(|item| item.client.is_none());
    }
    if range.inline {
        items.retain(|item| item.skill);
    }
    filter_items(&items, &range.query)
        .into_iter()
        .cloned()
        .collect()
}

/// `scoreMatch` tiers, best first (`protocol/src/search/text-match.ts`).
const TIER_EXACT: u8 = 0;
const TIER_WHOLE_WORD: u8 = 1;
const TIER_PREFIX: u8 = 2;
const TIER_WORD_START: u8 = 3;
const TIER_SUBSTRING: u8 = 4;
const TIER_SUBSEQUENCE: u8 = 5;

/// A match tier and the byte offset it was found at; lower is better on both.
type Score = (u8, usize);

fn is_word_boundary(c: Option<char>) -> bool {
    c.is_none_or(|c| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
}

/// `scoreMatch` without typo tolerance, on lowercase `query` and `text`.
fn score_match(query: &str, text: &str) -> Option<Score> {
    if text == query {
        return Some((TIER_EXACT, 0));
    }
    let mut best: Option<Score> = None;
    for (found, _) in text.match_indices(query) {
        let before = text[..found].chars().next_back();
        let after = text[found + query.len()..].chars().next();
        let starts = found == 0 || is_word_boundary(before);
        let ends = is_word_boundary(after);
        let tier = match (starts, ends) {
            (true, true) => TIER_WHOLE_WORD,
            _ if found == 0 => TIER_PREFIX,
            (true, false) => TIER_WORD_START,
            _ => TIER_SUBSTRING,
        };
        if best.is_none_or(|best| (tier, found) < best) {
            best = Some((tier, found));
        }
    }
    best.or_else(|| score_subsequence(query, text))
}

/// The query's characters in order inside one whitespace-delimited word.
fn score_subsequence(query: &str, text: &str) -> Option<Score> {
    let mut wanted = query.chars().peekable();
    let mut first = None;
    for (at, c) in text.char_indices() {
        if c.is_whitespace() {
            wanted = query.chars().peekable();
            first = None;
            continue;
        }
        if wanted.peek() == Some(&c) {
            first.get_or_insert(at);
            wanted.next();
        }
        if wanted.peek().is_none() {
            return first.map(|first| (TIER_SUBSEQUENCE, first));
        }
    }
    None
}

/// The items that match `query`, best first, as `filterAndRankCommandAutocompleteEntries`: the
/// best score of the name and aliases, then the name. An empty query keeps every item in order.
pub fn filter_items<'a>(items: &'a [SlashItem], query: &str) -> Vec<&'a SlashItem> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return items.iter().collect();
    }
    let mut ranked: Vec<(Score, &SlashItem)> = items
        .iter()
        .filter_map(|item| {
            std::iter::once(item.name.as_str())
                .chain(item.aliases.iter().map(String::as_str))
                .filter_map(|field| score_match(&query, &field.to_lowercase()))
                .min()
                .map(|score| (score, item))
        })
        .collect();
    ranked.sort_by(|(a, a_item), (b, b_item)| {
        a.cmp(b).then_with(|| {
            let (a_name, b_name) = (a_item.name.to_lowercase(), b_item.name.to_lowercase());
            a_name
                .cmp(&b_name)
                .then_with(|| a_item.name.cmp(&b_item.name))
        })
    });
    ranked.into_iter().map(|(_, item)| item).collect()
}

/// What picking an item puts in the input. A daemon command takes arguments, so it ends with a
/// space; a client command takes none.
pub fn insert_text(item: &SlashItem) -> String {
    if item.client.is_some() {
        format!("/{}", item.name)
    } else {
        format!("/{} ", item.name)
    }
}

/// `applySlashCommandReplacement`: `item` replaces the typed `/query`, with a trailing space
/// only at the end of the text. Returns the new text and the caret after the name.
pub fn replace_slash(text: &str, range: &SlashRange, item: &SlashItem) -> (String, usize) {
    let mut next = format!("{}/{}", &text[..range.start], item.name);
    let mut caret = next.len();
    next.push_str(&text[range.end..]);
    if range.end == text.len() && item.client.is_none() {
        next.push(' ');
        caret += 1;
    }
    (next, caret)
}

/// The client command that `text` is, as `resolveClientSlashCommand` does: no attachment, the
/// whole trimmed text is `/name` or `/alias`.
pub fn resolve_client_command(text: &str, attachments: usize) -> Option<ClientCommand> {
    if attachments > 0 {
        return None;
    }
    let name = text.trim().strip_prefix('/')?;
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return None;
    }
    client_items()
        .into_iter()
        .find(|item| item.name == name || item.aliases.iter().any(|alias| alias == name))
        .and_then(|item| item.client)
}

/// The step an arrow key makes through the matches, best first. The menu draws the best match
/// at the bottom, next to the input (`orderAutocompleteOptions` "above-input"), so Up moves to
/// the next worse match.
pub fn arrow_delta(key: &str) -> Option<isize> {
    match key {
        "up" => Some(1),
        "down" => Some(-1),
        _ => None,
    }
}

/// The key a menu step moves to: `delta` rows from `active`, wrapping at both ends.
pub fn step(active: usize, len: usize, delta: isize) -> usize {
    if len == 0 {
        return 0;
    }
    (active as isize + delta).rem_euclid(len as isize) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn daemon(names: &[&str]) -> Vec<DaemonCommand> {
        names.iter().map(|name| command(name, false)).collect()
    }

    fn command(name: &str, skill: bool) -> DaemonCommand {
        DaemonCommand {
            name: name.to_string(),
            description: format!("{name} description"),
            skill,
        }
    }

    fn names(items: &[&SlashItem]) -> Vec<String> {
        items.iter().map(|item| item.name.clone()).collect()
    }

    fn range(text: &str) -> SlashRange {
        find_active_slash(text, text.len()).expect("a slash command")
    }

    fn matched(daemon: &[DaemonCommand], text: &str, draft: bool) -> Vec<String> {
        menu_matches(daemon, &[], &range(text), draft)
            .into_iter()
            .map(|item| item.name)
            .collect()
    }

    #[test]
    fn the_query_is_the_text_after_a_slash_at_the_start() {
        let at = |text: &str| find_active_slash(text, text.len());
        assert_eq!(
            at("/").map(|r| (r.query, r.inline)),
            Some((String::new(), false))
        );
        assert_eq!(at("/re").map(|r| r.query), Some("re".to_string()));
        assert_eq!(at("/review now"), None);
        assert_eq!(at("/review\n"), None);
        assert_eq!(at(""), None);
        assert_eq!(at("a/b"), None);
        assert_eq!(at("say \"/re"), None);
    }

    #[test]
    fn a_slash_after_whitespace_is_inline() {
        let inline = range("fix this /re");
        assert_eq!(
            (inline.start, inline.query.as_str(), inline.inline),
            (9, "re", true)
        );
        assert!(range("line\n/re").inline);
        // A path slash is skipped, and the earlier slash is no command once a `/` follows it.
        assert_eq!(find_active_slash("see /a/b", 8), None);
        assert_eq!(
            find_active_slash("see /a/b", 6).map(|r| r.query),
            Some("a".to_string())
        );
    }

    #[test]
    fn the_caret_picks_the_command_not_the_end_of_the_text() {
        let text = "/review now";
        let at = find_active_slash(text, 3).expect("caret inside the name");
        assert_eq!((at.query.as_str(), at.start, at.end), ("re", 0, 3));
        assert_eq!(find_active_slash(text, 11), None);
        // A caret past the end, or inside a multi-byte character, is clamped.
        assert_eq!(find_active_slash("/é", 2).map(|r| r.end), Some(1));
        assert_eq!(
            find_active_slash("/ab", 99).map(|r| r.query),
            Some("ab".to_string())
        );
    }

    #[test]
    fn an_empty_query_lists_everything_client_commands_first() {
        let items = menu_items(&daemon(&["review", "init"]));
        let all = filter_items(&items, "");
        assert_eq!(names(&all), ["exit", "clear", "review", "init"]);
    }

    #[test]
    fn a_client_command_hides_a_daemon_command_of_its_name_or_alias() {
        let items = menu_items(&daemon(&["clear", "review", "q"]));
        let all = filter_items(&items, "");
        assert_eq!(names(&all), ["exit", "clear", "review"]);
        assert!(
            items
                .iter()
                .find(|i| i.name == "clear")
                .unwrap()
                .client
                .is_some()
        );
    }

    #[test]
    fn a_skill_named_like_a_built_in_stays_hidden() {
        let skills = [command("clear", true), command("deploy", true)];
        assert_eq!(matched(&skills, "x /cl", false), Vec::<String>::new());
        assert_eq!(matched(&skills, "/cl", true), Vec::<String>::new());
        assert_eq!(matched(&skills, "/", false), ["exit", "clear", "deploy"]);
    }

    #[test]
    fn filtering_ranks_exact_then_prefix_then_word_start_then_substring() {
        let items = menu_items(&daemon(&[
            "compact",
            "pre-commit",
            "commit",
            "quit-now",
            "recommend",
        ]));
        // Prefix matches tie and sort by name; `pre-commit` starts a word; `recommend` does not.
        assert_eq!(
            names(&filter_items(&items, "com")),
            ["commit", "compact", "pre-commit", "recommend"]
        );
        // `q` is an alias of /exit, so it is an exact match; quit-now is a prefix.
        assert_eq!(names(&filter_items(&items, "q")), ["exit", "quit-now"]);
        assert_eq!(names(&filter_items(&items, "NEW")), ["clear"]);
        assert!(filter_items(&items, "zzz").is_empty());
    }

    #[test]
    fn filtering_takes_a_subsequence_of_one_word() {
        let items = menu_items(&daemon(&["paseo-babysit", "review"]));
        assert_eq!(names(&filter_items(&items, "pasbab")), ["paseo-babysit"]);
        assert_eq!(names(&filter_items(&items, "rview")), ["review"]);
    }

    #[test]
    fn a_slash_after_text_lists_skills_only() {
        let daemon = [
            command("review", false),
            command("deploy", true),
            command("lint", true),
        ];
        assert_eq!(matched(&daemon, "x /", false), ["deploy", "lint"]);
        assert_eq!(matched(&daemon, "x /li", false), ["lint"]);
        assert_eq!(
            matched(&daemon, "/", false),
            ["exit", "clear", "review", "deploy", "lint"]
        );
    }

    #[test]
    fn a_slash_after_text_hides_plugin_commands() {
        let plugins = vec![("route".to_string(), "Route".to_string())];
        let daemon = [command("deploy", true)];
        let inline = menu_matches(&daemon, &plugins, &range("go /"), false);
        assert_eq!(inline.len(), 1);
        let start = menu_matches(&daemon, &plugins, &range("/"), false);
        assert!(start.iter().any(|item| item.plugin));
    }

    #[test]
    fn a_draft_lists_no_built_in_command() {
        let daemon = [command("review", false), command("deploy", true)];
        assert_eq!(matched(&daemon, "/", true), ["review", "deploy"]);
        // A draft whose list failed or is empty still opens, with plugin commands alone.
        let plugins = vec![("route".to_string(), "Route".to_string())];
        let items = menu_matches(&[], &plugins, &range("/"), true);
        assert_eq!(
            items.iter().map(|i| i.name.as_str()).collect::<Vec<_>>(),
            ["route"]
        );
        assert!(menu_matches(&[], &[], &range("/"), true).is_empty());
    }

    #[test]
    fn an_empty_daemon_list_keeps_the_built_in_commands() {
        assert_eq!(matched(&[], "/", false), ["exit", "clear"]);
    }

    #[test]
    fn picking_a_daemon_command_leaves_room_for_arguments() {
        let items = menu_items(&daemon(&["review"]));
        assert_eq!(insert_text(&items[0]), "/exit");
        assert_eq!(insert_text(&items[2]), "/review ");
    }

    #[test]
    fn picking_replaces_the_typed_command_in_place() {
        let items = menu_items(&[command("deploy", true)]);
        let deploy = &items[2];
        assert_eq!(
            replace_slash("/dep", &range("/dep"), deploy),
            ("/deploy ".to_string(), 8)
        );
        assert_eq!(
            replace_slash("fix /de", &range("fix /de"), deploy),
            ("fix /deploy ".to_string(), 12)
        );
        let text = "fix /de now";
        let mid = find_active_slash(text, 7).expect("inline range");
        assert_eq!(
            replace_slash(text, &mid, deploy),
            ("fix /deploy now".to_string(), 11)
        );
        assert_eq!(
            replace_slash("/ex", &range("/ex"), &items[0]),
            ("/exit".to_string(), 5)
        );
    }

    #[test]
    fn a_client_command_resolves_by_name_or_alias_without_attachments() {
        assert_eq!(
            resolve_client_command("/exit", 0),
            Some(ClientCommand::Exit)
        );
        assert_eq!(
            resolve_client_command(" /q \n", 0),
            Some(ClientCommand::Exit)
        );
        assert_eq!(
            resolve_client_command("/new", 0),
            Some(ClientCommand::Clear)
        );
        assert_eq!(resolve_client_command("/exit", 1), None);
        assert_eq!(resolve_client_command("/exit now", 0), None);
        assert_eq!(resolve_client_command("/review", 0), None);
        assert_eq!(resolve_client_command("/", 0), None);
        assert_eq!(resolve_client_command("exit", 0), None);
    }

    #[test]
    fn plugin_commands_sit_between_built_ins_and_daemon_commands() {
        let plugins = vec![
            ("clear".to_string(), "plugin clear".to_string()),
            ("route".to_string(), "Route the turn".to_string()),
        ];
        let items = merge_items(&daemon(&["review", "route"]), &plugins);
        let all = filter_items(&items, "");
        // The plugin `/clear` hides behind the built-in; the daemon `/route` behind the plugin.
        assert_eq!(names(&all), ["exit", "clear", "route", "review"]);
        let route = items.iter().find(|item| item.name == "route").unwrap();
        assert!(route.plugin && route.client.is_none());
        assert_eq!(insert_text(route), "/route ");
        let clear = items.iter().find(|item| item.name == "clear").unwrap();
        assert!(clear.client.is_some() && !clear.plugin);
    }

    #[test]
    fn a_plugin_command_takes_its_trimmed_arguments_without_attachments() {
        let names = ["route"];
        assert_eq!(
            resolve_plugin_command("/route fast  ", 0, &names),
            Some(("route", "fast".to_string()))
        );
        assert_eq!(
            resolve_plugin_command(" /route", 0, &names),
            Some(("route", String::new()))
        );
        assert_eq!(resolve_plugin_command("/route fast", 1, &names), None);
        assert_eq!(resolve_plugin_command("/other fast", 0, &names), None);
        assert_eq!(resolve_plugin_command("route fast", 0, &names), None);
    }

    #[test]
    fn up_moves_away_from_the_best_match_at_the_bottom() {
        // Matches best first: 0 is the bottom row, drawn next to the input.
        assert_eq!(step(0, 3, arrow_delta("up").unwrap()), 1);
        assert_eq!(step(0, 3, arrow_delta("down").unwrap()), 2);
        assert_eq!(arrow_delta("tab"), None);
    }

    #[test]
    fn stepping_wraps_at_both_ends() {
        assert_eq!(step(0, 3, 1), 1);
        assert_eq!(step(2, 3, 1), 0);
        assert_eq!(step(0, 3, -1), 2);
        assert_eq!(step(0, 0, 1), 0);
    }
}
