use bachuc_command::search::{Row, SearchIndex};
use std::time::{Duration, Instant};

fn sample(count: usize) -> Vec<(String, String)> {
    let verbs = [
        "Open", "Close", "Toggle", "Split", "Focus", "Archive", "Rename", "Cycle",
    ];
    let nouns = [
        "workspace",
        "tab",
        "pane",
        "sidebar",
        "terminal",
        "agent",
        "theme",
        "host",
    ];
    (0..count)
        .map(|n| {
            (
                format!(
                    "{} {} {}",
                    verbs[n % verbs.len()],
                    nouns[(n / verbs.len()) % nouns.len()],
                    n
                ),
                format!("Group {}", n % 12),
            )
        })
        .collect()
}

fn index(items: &[(String, String)]) -> SearchIndex {
    SearchIndex::new(items.iter().map(|(t, g)| (t.as_str(), g.as_str())))
}

#[test]
fn fuzzy_ranking_orders_by_match_quality() {
    let items: Vec<(String, String)> = [
        "Toggle command center",
        "Command",
        "Recommend a theme",
        "Cycle theme",
    ]
    .iter()
    .map(|t| (t.to_string(), "Actions".to_string()))
    .collect();
    let index = index(&items);
    let layout = index.layout("comm");
    let order: Vec<&str> = layout.order.iter().map(|i| items[*i].0.as_str()).collect();
    // A prefix of the text, then the start of a word, then the inside of a word.
    assert_eq!(
        order,
        ["Command", "Toggle command center", "Recommend a theme"]
    );
    // Characters in order inside one word; an earlier match ranks first.
    let order: Vec<&str> = index
        .layout("cmd")
        .order
        .iter()
        .map(|i| items[*i].0.as_str())
        .collect();
    assert_eq!(
        order,
        ["Command", "Recommend a theme", "Toggle command center"]
    );
}

#[test]
fn query_without_a_match_has_no_rows() {
    let items = sample(50);
    let layout = index(&items).layout("zzzzqq");
    assert!(layout.rows.is_empty());
    assert!(layout.order.is_empty());
}

#[test]
fn query_keeps_sections_and_rows_consistent() {
    let items = sample(200);
    let layout = index(&items).layout("tab");
    assert!(!layout.order.is_empty());
    for (position, item) in layout.order.iter().enumerate() {
        assert_eq!(
            layout.rows[layout.row_of[position]],
            Row::Item { item: *item }
        );
    }
    // The first section has no divider; the rest have one.
    let mut sections = layout.rows.iter().filter_map(|row| match row {
        Row::Section { divider, .. } => Some(*divider),
        _ => None,
    });
    assert_eq!(sections.next(), Some(false));
    assert!(sections.all(|divider| divider));
}

/// Anti-goal of TASK-176: 500 app items plus 100 plugin items that carry keywords. Half the
/// typed queries match only through a keyword, which takes the slowest scoring path.
#[test]
fn search_600_items_with_keywords_stays_under_16_ms_per_keystroke() {
    let items = sample(500);
    let plugin: Vec<(String, String, Vec<String>)> = (0..100)
        .map(|n| {
            (
                format!("Plugin action {n}"),
                format!("plugin-{}", n % 4),
                vec![
                    format!("diff{n}"),
                    "review".to_string(),
                    "compare".to_string(),
                ],
            )
        })
        .collect();
    let index = SearchIndex::with_keywords(
        plugin
            .iter()
            .map(|(t, g, k)| {
                (
                    t.as_str(),
                    g.as_str(),
                    k.iter().map(String::as_str).collect(),
                )
            })
            .chain(
                items
                    .iter()
                    .map(|(t, g)| (t.as_str(), g.as_str(), Vec::new())),
            ),
    );
    let mut worst = Duration::ZERO;
    for typed in ["toggle sidebar 4", "compare review", "zzzz nothing"] {
        for _ in 0..5 {
            for end in 1..=typed.len() {
                let started = Instant::now();
                let layout = index.layout(&typed[..end]);
                worst = worst.max(started.elapsed());
                std::hint::black_box(layout);
            }
        }
    }
    println!("worst keystroke over 600 items with keywords: {worst:?}");
    assert!(
        worst < Duration::from_millis(16),
        "worst keystroke took {worst:?}"
    );
}

/// Anti-goal of TASK-152: a keystroke over 500 items takes at most 16 ms. This runs unoptimized,
/// the slower case. The test keeps the worst of many keystrokes of a typed query.
#[test]
fn search_500_items_stays_under_16_ms_per_keystroke() {
    let items = sample(500);
    let index = index(&items);
    let typed = "toggle sidebar 4";
    let mut worst = Duration::ZERO;
    for _ in 0..5 {
        for end in 1..=typed.len() {
            let started = Instant::now();
            let layout = index.layout(&typed[..end]);
            worst = worst.max(started.elapsed());
            std::hint::black_box(layout);
        }
    }
    println!("worst keystroke over 500 items: {worst:?}");
    assert!(
        worst < Duration::from_millis(16),
        "worst keystroke took {worst:?}"
    );
}
