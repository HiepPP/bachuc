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
pub fn menu_items(daemon: &[SlashCommand]) -> Vec<SlashItem> {
    let mut items = client_items();
    let claimed = |name: &str| {
        items
            .iter()
            .any(|item| item.name == name || item.aliases.iter().any(|alias| alias == name))
    };
    let daemon: Vec<SlashItem> = daemon
        .iter()
        .filter(|command| !claimed(&command.name))
        .map(|command| SlashItem {
            name: command.name.clone(),
            description: command.description.clone(),
            aliases: Vec::new(),
            client: None,
        })
        .collect();
    items.extend(daemon);
    items
}

/// The text after the leading `/` while the user is still typing the command name. `None` when
/// the text does not start with `/`, or already holds whitespace (an argument).
pub fn slash_query(text: &str) -> Option<&str> {
    let rest = text.strip_prefix('/')?;
    (!rest.chars().any(char::is_whitespace)).then_some(rest)
}

/// The items that match `query`, best first: names that start with it, then names and aliases
/// that contain it. An empty query keeps every item in order.
pub fn filter_items<'a>(items: &'a [SlashItem], query: &str) -> Vec<&'a SlashItem> {
    let query = query.to_lowercase();
    let rank = |item: &SlashItem| -> Option<u8> {
        let name = item.name.to_lowercase();
        if name.starts_with(&query) {
            Some(0)
        } else if item
            .aliases
            .iter()
            .any(|alias| alias.to_lowercase().starts_with(&query))
        {
            Some(1)
        } else if name.contains(&query) {
            Some(2)
        } else {
            None
        }
    };
    let mut ranked: Vec<(u8, usize, &SlashItem)> = items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| rank(item).map(|rank| (rank, index, item)))
        .collect();
    ranked.sort_by_key(|(rank, index, _)| (*rank, *index));
    ranked.into_iter().map(|(_, _, item)| item).collect()
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

    fn daemon(names: &[&str]) -> Vec<SlashCommand> {
        names
            .iter()
            .map(|name| SlashCommand {
                name: name.to_string(),
                description: format!("{name} description"),
            })
            .collect()
    }

    fn names(items: &[&SlashItem]) -> Vec<String> {
        items.iter().map(|item| item.name.clone()).collect()
    }

    #[test]
    fn the_query_is_the_text_after_a_leading_slash() {
        assert_eq!(slash_query("/"), Some(""));
        assert_eq!(slash_query("/re"), Some("re"));
        assert_eq!(slash_query("/review now"), None);
        assert_eq!(slash_query("/review\n"), None);
        assert_eq!(slash_query("hello /re"), None);
        assert_eq!(slash_query(""), None);
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
    fn filtering_ranks_prefix_then_alias_then_substring() {
        let items = menu_items(&daemon(&["compact", "pre-commit", "commit", "quit-now"]));
        assert_eq!(
            names(&filter_items(&items, "com")),
            ["compact", "commit", "pre-commit"]
        );
        // `q` is an alias of /exit, and a prefix of quit-now.
        assert_eq!(names(&filter_items(&items, "q")), ["quit-now", "exit"]);
        assert_eq!(names(&filter_items(&items, "NEW")), ["clear"]);
        assert!(filter_items(&items, "zzz").is_empty());
    }

    #[test]
    fn picking_a_daemon_command_leaves_room_for_arguments() {
        let items = menu_items(&daemon(&["review"]));
        assert_eq!(insert_text(&items[0]), "/exit");
        assert_eq!(insert_text(&items[2]), "/review ");
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
