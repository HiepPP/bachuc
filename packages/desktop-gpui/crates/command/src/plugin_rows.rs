//! The plugin rows of the command center, as `buildPluginCommandCenterContributions` builds them
//! in `plugins/command-center/contributions.ts`. Every rule is a plain function, because a view
//! crate has no headless GPUI context to test in.

use crate::center::CommandItem;
use crate::search::{Layout, SearchIndex};
use bachuc_icons::IconName;
use bachuc_plugin_host::{CommandCenterItem, CommandContext, CommandInput, Contributed, Handler};
use gpui_kit::SharedString;

/// One plugin item that the context and `visible` let through.
#[derive(Clone)]
pub(crate) struct PluginRow {
    pub id: String,
    pub plugin_id: &'static str,
    pub title: SharedString,
    pub icon: IconName,
    /// Hidden search text. Never drawn.
    pub keywords: Vec<SharedString>,
    pub on_select: Handler<CommandInput>,
}

impl PluginRow {
    /// The list row of this item, in the section of its plugin.
    pub fn item(&self) -> CommandItem {
        CommandItem {
            id: self.id.clone(),
            title: self.title.clone(),
            group: self.plugin_id.into(),
            icon: Some(self.icon),
            shortcut: None,
        }
    }

    fn same_look(&self, other: &PluginRow) -> bool {
        self.id == other.id
            && self.plugin_id == other.plugin_id
            && self.title == other.title
            && self.icon == other.icon
            && self.keywords == other.keywords
    }
}

/// `plugin:<plugin id>:<item id>`. The app ignores an id with this prefix.
pub(crate) fn row_id(plugin_id: &str, item_id: &str) -> String {
    format!("plugin:{plugin_id}:{item_id}")
}

/// `Global` always; `Workspace` with a workspace id; `Agent` with a workspace id and an agent id.
pub(crate) fn context_allows(
    context: CommandContext,
    workspace_id: Option<&str>,
    agent_id: Option<&str>,
) -> bool {
    match context {
        CommandContext::Global => true,
        CommandContext::Workspace => workspace_id.is_some(),
        CommandContext::Agent => workspace_id.is_some() && agent_id.is_some(),
    }
}

/// The rows for the host's command items, in index order. The context gate runs first, then
/// `visible`, so `visible` never runs for an item the context hides.
pub(crate) fn plugin_rows(
    items: &[Contributed<CommandCenterItem>],
    workspace_id: Option<&str>,
    agent_id: Option<&str>,
    visible: impl Fn(&CommandCenterItem) -> bool,
) -> Vec<PluginRow> {
    items
        .iter()
        .filter(|entry| context_allows(entry.value.context, workspace_id, agent_id))
        .filter(|entry| visible(&entry.value))
        .map(|entry| PluginRow {
            id: row_id(entry.plugin_id, entry.value.id),
            plugin_id: entry.plugin_id,
            title: entry.value.title.clone(),
            icon: entry.value.icon,
            keywords: entry.value.keywords.clone(),
            on_select: entry.value.on_select.clone(),
        })
        .collect()
}

/// Whether two row lists draw and search the same. A handler does not count.
pub(crate) fn same_rows(left: &[PluginRow], right: &[PluginRow]) -> bool {
    left.len() == right.len() && left.iter().zip(right).all(|(l, r)| l.same_look(r))
}

/// The plugin rows, then the app's items.
pub(crate) fn combine(rows: &[PluginRow], app_items: &[CommandItem]) -> Vec<CommandItem> {
    rows.iter()
        .map(PluginRow::item)
        .chain(app_items.iter().cloned())
        .collect()
}

/// The index of `items`, where the first `rows.len()` items are the plugin rows.
pub(crate) fn index_for(rows: &[PluginRow], items: &[CommandItem]) -> SearchIndex {
    SearchIndex::with_keywords(items.iter().enumerate().map(|(position, item)| {
        let keywords = rows
            .get(position)
            .map(|row| row.keywords.iter().map(|k| k.as_ref()).collect())
            .unwrap_or_default();
        (item.title.as_ref(), item.group.as_ref(), keywords)
    }))
}

/// The layout of `query` and the position to highlight: the row of `selected` when the layout
/// still has it, else the first row.
pub(crate) fn layout_keeping(
    index: &SearchIndex,
    items: &[CommandItem],
    query: &str,
    selected: Option<&str>,
) -> (Layout, Option<usize>) {
    let layout = index.layout(query);
    let kept = selected.and_then(|id| {
        layout
            .order
            .iter()
            .position(|item| items.get(*item).is_some_and(|item| item.id == id))
    });
    let active = kept.or((!layout.order.is_empty()).then_some(0));
    (layout, active)
}

/// Runs a plugin row: the handler first, then one `Run` event. The handler may move focus or
/// open a panel, so the event, which closes the palette, comes after it.
pub(crate) fn run_row<C, E>(
    cx: &mut C,
    event: E,
    call: impl FnOnce(&mut C),
    emit: impl FnOnce(&mut C, E),
) {
    call(cx);
    emit(cx, event);
}

#[cfg(test)]
mod tests {
    use super::{
        PluginRow, combine, context_allows, index_for, layout_keeping, plugin_rows, row_id,
        run_row, same_rows,
    };
    use crate::center::CommandItem;
    use bachuc_icons::IconName;
    use bachuc_plugin_host::{CommandCenterItem, CommandContext, CommandInput, Contributed};
    use std::cell::Cell;
    use std::rc::Rc;

    fn item(
        plugin_id: &'static str,
        id: &'static str,
        title: &str,
        context: CommandContext,
        keywords: &[&str],
    ) -> Contributed<CommandCenterItem> {
        Contributed {
            plugin_id,
            value: CommandCenterItem {
                id,
                title: title.to_string().into(),
                icon: IconName::Search,
                keywords: keywords.iter().map(|k| k.to_string().into()).collect(),
                context,
                visible: None,
                on_select: Rc::new(|_: CommandInput, _, _| {}),
            },
        }
    }

    fn app(id: &str, title: &str, group: &str) -> CommandItem {
        CommandItem {
            id: id.into(),
            title: title.to_string().into(),
            group: group.to_string().into(),
            icon: None,
            shortcut: None,
        }
    }

    fn ids(rows: &[PluginRow]) -> Vec<&str> {
        rows.iter().map(|row| row.id.as_str()).collect()
    }

    #[test]
    fn row_id_has_the_plugin_prefix() {
        assert_eq!(row_id("diff-tools", "open"), "plugin:diff-tools:open");
    }

    #[test]
    fn no_plugin_item_leaves_the_list_as_it_is() {
        let rows = plugin_rows(&[], None, None, |_| true);
        let app_items = vec![
            app("action:a", "A", "Actions"),
            app("agent:1", "B", "Agents"),
        ];
        assert!(rows.is_empty());
        assert_eq!(combine(&rows, &app_items), app_items);
    }

    #[test]
    fn the_context_gate() {
        let items = [
            item("p", "global", "Global", CommandContext::Global, &[]),
            item(
                "p",
                "workspace",
                "Workspace",
                CommandContext::Workspace,
                &[],
            ),
            item("p", "agent", "Agent", CommandContext::Agent, &[]),
        ];
        let show = |ws, ag| ids(&plugin_rows(&items, ws, ag, |_| true)).join(",");
        assert_eq!(show(None, None), "plugin:p:global");
        assert_eq!(show(None, Some("a")), "plugin:p:global");
        assert_eq!(show(Some("w"), None), "plugin:p:global,plugin:p:workspace");
        assert_eq!(
            show(Some("w"), Some("a")),
            "plugin:p:global,plugin:p:workspace,plugin:p:agent"
        );
        assert!(!context_allows(CommandContext::Agent, None, Some("a")));
    }

    #[test]
    fn visible_false_hides_and_runs_after_the_context_gate() {
        let items = [
            item("p", "a", "A", CommandContext::Global, &[]),
            item("p", "b", "B", CommandContext::Workspace, &[]),
            item("p", "c", "C", CommandContext::Global, &[]),
        ];
        let calls = Cell::new(0);
        let rows = plugin_rows(&items, None, None, |entry| {
            calls.set(calls.get() + 1);
            entry.id != "a"
        });
        assert_eq!(ids(&rows), ["plugin:p:c"]);
        // The workspace item never reached `visible`.
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn two_plugins_make_two_sections_before_the_app_sections() {
        let items = [
            item("one", "x", "X", CommandContext::Global, &[]),
            item("two", "y", "Y", CommandContext::Global, &[]),
            item("one", "z", "Z", CommandContext::Global, &[]),
        ];
        let rows = plugin_rows(&items, None, None, |_| true);
        let all = combine(&rows, &[app("action:a", "A", "Actions")]);
        let index = index_for(&rows, &all);
        let layout = index.layout("");
        let order: Vec<&str> = layout.order.iter().map(|i| all[*i].id.as_str()).collect();
        assert_eq!(
            order,
            ["plugin:one:x", "plugin:one:z", "plugin:two:y", "action:a"]
        );
        let titles: Vec<Option<String>> = layout
            .rows
            .iter()
            .filter_map(|row| match row {
                crate::search::Row::Section { title, .. } => Some(title.clone()),
                _ => None,
            })
            .collect();
        assert_eq!(
            titles,
            [
                Some("one".into()),
                Some("two".into()),
                Some("Actions".into())
            ]
        );
    }

    #[test]
    fn a_title_match_outranks_a_keyword_match_and_the_keyword_is_not_drawn() {
        let items = [
            item("p", "b", "Compare files", CommandContext::Global, &["diff"]),
            item("p", "a", "Diff view", CommandContext::Global, &[]),
        ];
        let rows = plugin_rows(&items, None, None, |_| true);
        let all = combine(&rows, &[]);
        let layout = index_for(&rows, &all).layout("diff");
        let order: Vec<&str> = layout
            .order
            .iter()
            .map(|i| all[*i].title.as_ref())
            .collect();
        assert_eq!(order, ["Diff view", "Compare files"]);
        assert!(
            all.iter()
                .all(|item| !item.title.contains("diff") || item.title.as_ref() == "Diff view")
        );
    }

    #[test]
    fn a_keyword_match_ranks_below_a_group_match_of_an_app_item() {
        let items = [item("p", "k", "Zed", CommandContext::Global, &["tabs"])];
        let rows = plugin_rows(&items, None, None, |_| true);
        let all = combine(&rows, &[app("action:t", "Alpha", "Tabs")]);
        let layout = index_for(&rows, &all).layout("tabs");
        let order: Vec<&str> = layout.order.iter().map(|i| all[*i].id.as_str()).collect();
        assert_eq!(order, ["action:t", "plugin:p:k"]);
    }

    #[test]
    fn a_rebuild_keeps_the_query_and_the_selected_row() {
        let before = [item("p", "edit", "Edit file", CommandContext::Global, &[])];
        let rows = plugin_rows(&before, None, None, |_| true);
        let app_items = [app("action:e", "Open editor", "Actions")];
        let all = combine(&rows, &app_items);
        let (layout, active) =
            layout_keeping(&index_for(&rows, &all), &all, "ed", Some("action:e"));
        assert_eq!(all[layout.order[active.unwrap()]].id, "action:e");

        // A second plugin item arrives while "ed" is typed and "action:e" is highlighted.
        let after = [
            item("p", "edit", "Edit file", CommandContext::Global, &[]),
            item("q", "edge", "Edge cases", CommandContext::Global, &[]),
        ];
        let rows = plugin_rows(&after, None, None, |_| true);
        let all = combine(&rows, &app_items);
        let (layout, active) =
            layout_keeping(&index_for(&rows, &all), &all, "ed", Some("action:e"));
        assert_eq!(layout.order.len(), 3);
        assert_eq!(all[layout.order[active.unwrap()]].id, "action:e");
    }

    #[test]
    fn a_rebuild_falls_back_to_the_first_row_when_the_selection_is_gone() {
        let items = [item("p", "a", "Alpha", CommandContext::Global, &[])];
        let rows = plugin_rows(&items, None, None, |_| true);
        let all = combine(&rows, &[]);
        let index = index_for(&rows, &all);
        assert_eq!(
            layout_keeping(&index, &all, "", Some("plugin:p:gone")).1,
            Some(0)
        );
        assert_eq!(
            layout_keeping(&index, &all, "zzz", Some("plugin:p:a")).1,
            None
        );
    }

    #[test]
    fn same_rows_ignores_the_handler_but_not_the_look() {
        let a = plugin_rows(
            &[item("p", "a", "A", CommandContext::Global, &["k"])],
            None,
            None,
            |_| true,
        );
        let b = plugin_rows(
            &[item("p", "a", "A", CommandContext::Global, &["k"])],
            None,
            None,
            |_| true,
        );
        let renamed = plugin_rows(
            &[item("p", "a", "B", CommandContext::Global, &["k"])],
            None,
            None,
            |_| true,
        );
        let other_keywords = plugin_rows(
            &[item("p", "a", "A", CommandContext::Global, &["j"])],
            None,
            None,
            |_| true,
        );
        assert!(same_rows(&a, &b));
        assert!(!same_rows(&a, &renamed));
        assert!(!same_rows(&a, &other_keywords));
        assert!(!same_rows(&a, &[]));
    }

    #[test]
    fn a_run_calls_the_handler_then_emits_once() {
        let mut log: Vec<String> = Vec::new();
        run_row(
            &mut log,
            "plugin:p:a".to_string(),
            |log| log.push("handler".into()),
            |log, id| log.push(format!("run {id}")),
        );
        assert_eq!(log, ["handler", "run plugin:p:a"]);
    }
}
