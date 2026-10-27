//! The cases of `packages/app/src/assistant-file-links/parse.test.ts`, plus the code span
//! linkifier that feeds them. `normalizeInlinePathTarget` has no port: the timeline emits the
//! resolved path and the app's file pane places it.

use bachuc_timeline::links::{
    FileTarget, LinkClick, LinkKind, classify, is_file_looking_token, linkify_code_spans,
    parse_file_link, parse_file_protocol_url, parse_inline_path_token, resolve_click,
};

const ROOT: Option<&str> = Some("/Users/test/project");

fn target(raw: &str, path: &str, start: Option<u32>, end: Option<u32>) -> FileTarget {
    FileTarget {
        raw: raw.into(),
        path: path.into(),
        line_start: start,
        line_end: end,
    }
}

#[test]
fn inline_tokens_need_a_line() {
    assert_eq!(parse_inline_path_token("src/app.ts"), None);
    assert_eq!(parse_inline_path_token("README.md"), None);
    assert_eq!(
        parse_inline_path_token("src/app.ts:12"),
        Some(target("src/app.ts:12", "src/app.ts", Some(12), None))
    );
    assert_eq!(
        parse_inline_path_token("src/app.ts:12-20"),
        Some(target("src/app.ts:12-20", "src/app.ts", Some(12), Some(20)))
    );
    assert_eq!(
        parse_inline_path_token("src/app.ts:12:4"),
        Some(target("src/app.ts:12:4", "src/app.ts", Some(12), None))
    );
    assert_eq!(
        parse_inline_path_token("src/app.ts(12,4)"),
        Some(target("src/app.ts(12,4)", "src/app.ts", Some(12), None))
    );
    assert_eq!(
        parse_inline_path_token("src/app.ts lines 12-20"),
        Some(target(
            "src/app.ts lines 12-20",
            "src/app.ts",
            Some(12),
            Some(20)
        ))
    );
    assert_eq!(parse_inline_path_token(":12"), None);
    assert_eq!(parse_inline_path_token(":12-20"), None);
}

#[test]
fn file_urls_parse_with_line_fragments() {
    let url = "file:///Users/test/project/src/app.tsx#L81";
    assert_eq!(
        parse_file_protocol_url(url),
        Some(target(
            url,
            "/Users/test/project/src/app.tsx",
            Some(81),
            None
        ))
    );
    let url = "file:///Users/test/project/src/app.tsx#L81C5-L83C2";
    assert_eq!(
        parse_file_protocol_url(url),
        Some(target(
            url,
            "/Users/test/project/src/app.tsx",
            Some(81),
            Some(83)
        ))
    );
    let url = "file:///Users/test/project/src/app.tsx";
    assert_eq!(
        parse_file_protocol_url(url),
        Some(target(url, "/Users/test/project/src/app.tsx", None, None))
    );
    let url = "file:///C:/Users/test/project/src/app.tsx#L12-L20";
    assert_eq!(
        parse_file_protocol_url(url),
        Some(target(
            url,
            "C:/Users/test/project/src/app.tsx",
            Some(12),
            Some(20)
        ))
    );
    assert_eq!(
        parse_file_protocol_url("https://example.com/test.ts#L10"),
        None
    );
    assert_eq!(
        parse_file_protocol_url("file:///Users/test/project/src/app.tsx#L20-L12"),
        None
    );
}

#[test]
fn classify_keeps_external_urls_out() {
    assert_eq!(
        classify("http://dumm.md", ROOT),
        Some(LinkKind::External("http://dumm.md".into()))
    );
    assert_eq!(
        classify("mailto:test@example.com", None),
        Some(LinkKind::External("mailto:test@example.com".into()))
    );
}

#[test]
fn classify_splits_bare_candidates_from_relative_files() {
    assert_eq!(
        classify("dumm.md", ROOT),
        Some(LinkKind::AmbiguousFile(target(
            "dumm.md",
            "/Users/test/project/dumm.md",
            None,
            None
        )))
    );
    assert_eq!(
        classify("message-renderer.tsx", ROOT),
        Some(LinkKind::AmbiguousFile(target(
            "message-renderer.tsx",
            "/Users/test/project/message-renderer.tsx",
            None,
            None
        )))
    );
    assert_eq!(
        classify("src/components/message.tsx#L33", ROOT),
        Some(LinkKind::DirectFile(target(
            "src/components/message.tsx#L33",
            "/Users/test/project/src/components/message.tsx",
            Some(33),
            None
        )))
    );
}

#[test]
fn classify_ignores_domains_words_and_commands() {
    for value in ["google.com", "example.com", "openai.com/path"] {
        assert_eq!(classify(value, ROOT), None, "{value}");
    }
    for value in [
        "main",
        "origin/main",
        "1f7fc232b",
        "25994904967",
        "babysit main 1f7fc232b",
    ] {
        assert_eq!(classify(value, ROOT), None, "{value}");
    }
    assert_eq!(
        classify(
            "npm run lint -- packages/app/src/stores/workspace-layout-actions.ts packages/app/src/stores/workspace-layout-store.ts",
            ROOT
        ),
        None
    );
}

#[test]
fn parse_resolves_relative_and_absolute_links() {
    assert_eq!(
        parse_file_link("dumm.md", ROOT),
        Some(target("dumm.md", "/Users/test/project/dumm.md", None, None))
    );
    assert_eq!(
        parse_file_link("file.ts:12", ROOT),
        Some(target(
            "file.ts:12",
            "/Users/test/project/file.ts",
            Some(12),
            None
        ))
    );
    for value in ["google.com", "google.com:80", "openai.com/path"] {
        assert_eq!(parse_file_link(value, ROOT), None, "{value}");
    }
    assert_eq!(
        parse_file_link("src/components/message.tsx#L33", ROOT),
        Some(target(
            "src/components/message.tsx#L33",
            "/Users/test/project/src/components/message.tsx",
            Some(33),
            None
        ))
    );
    assert_eq!(
        parse_file_link("/Users/test/project/src/app.tsx#L33", ROOT),
        Some(target(
            "/Users/test/project/src/app.tsx#L33",
            "/Users/test/project/src/app.tsx",
            Some(33),
            None
        ))
    );
    assert_eq!(
        parse_file_link("/Users/test/project/src/app.tsx:33", ROOT),
        Some(target(
            "/Users/test/project/src/app.tsx:33",
            "/Users/test/project/src/app.tsx",
            Some(33),
            None
        ))
    );
    assert_eq!(
        parse_file_link("C:/repo/src/app.tsx#L12-L20", Some("C:/repo")),
        Some(target(
            "C:/repo/src/app.tsx#L12-L20",
            "C:/repo/src/app.tsx",
            Some(12),
            Some(20)
        ))
    );
    assert_eq!(
        parse_file_link("C:/repo/src/app.tsx:12-20", Some("C:/repo")),
        Some(target(
            "C:/repo/src/app.tsx:12-20",
            "C:/repo/src/app.tsx",
            Some(12),
            Some(20)
        ))
    );
}

#[test]
fn parse_allows_paths_outside_the_root_and_home_paths() {
    assert_eq!(
        parse_file_link("file:///tmp/outside.txt", ROOT),
        Some(target(
            "file:///tmp/outside.txt",
            "/tmp/outside.txt",
            None,
            None
        ))
    );
    assert_eq!(
        parse_file_link("/tmp/outside.txt", ROOT),
        Some(target("/tmp/outside.txt", "/tmp/outside.txt", None, None))
    );
    assert_eq!(
        parse_file_link("~/.paseo/plans/file-preview.md", ROOT),
        Some(target(
            "~/.paseo/plans/file-preview.md",
            "~/.paseo/plans/file-preview.md",
            None,
            None
        ))
    );
    assert_eq!(
        parse_file_link("~/.paseo/plans/file-preview.md:12", ROOT),
        Some(target(
            "~/.paseo/plans/file-preview.md:12",
            "~/.paseo/plans/file-preview.md",
            Some(12),
            None
        ))
    );
    assert_eq!(
        parse_file_link("~\\.paseo\\plans\\file-preview.md", ROOT),
        Some(target(
            "~\\.paseo\\plans\\file-preview.md",
            "~/.paseo/plans/file-preview.md",
            None,
            None
        ))
    );
}

#[test]
fn parse_rejects_external_urls_and_bad_ranges() {
    assert_eq!(
        parse_file_link("https://example.com/Users/test/project/src/app.tsx", None),
        None
    );
    assert_eq!(parse_file_link("http://dumm.md", ROOT), None);
    assert_eq!(
        parse_file_link("/Users/test/project/src/app.tsx#L20-L12", ROOT),
        None
    );
}

#[test]
fn parse_survives_literal_percent_signs() {
    for value in [
        "/tmp/100% packet loss",
        "/Users/test/project/0% off",
        "/var/log/%PATH%/x.log",
        "file:///tmp/100% packet loss",
    ] {
        let _ = parse_file_link(value, ROOT);
    }
    let _ = parse_file_protocol_url("file:///tmp/100% packet loss");
}

#[test]
fn code_spans_that_look_like_files_become_links() {
    assert!(is_file_looking_token("src/app.ts:12"));
    assert!(is_file_looking_token("README.md"));
    assert!(!is_file_looking_token("origin/main"));
    assert!(!is_file_looking_token("npm run lint"));

    let text = "See `src/app.ts:12` and `main`, and [`a.rs`](x).\n\n```\n`b.rs`\n```\n";
    assert_eq!(
        linkify_code_spans(text),
        "See [`src/app.ts:12`](<src/app.ts:12>) and `main`, and [`a.rs`](x).\n\n```\n`b.rs`\n```\n"
    );
}

#[test]
fn a_click_resolves_under_the_agent_folder() {
    assert_eq!(
        resolve_click("src/app.ts:12", "/repo"),
        LinkClick::File(target("src/app.ts:12", "/repo/src/app.ts", Some(12), None))
    );
    assert_eq!(
        resolve_click("https://paseo.sh", "/repo"),
        LinkClick::External("https://paseo.sh".into())
    );
    assert_eq!(resolve_click("main", "/repo"), LinkClick::Ignored);
}

#[test]
fn a_fence_meta_with_a_path_and_lines_is_a_code_citation() {
    let range = bachuc_timeline::links::code_citation("src/lib.rs:2331-2334").unwrap();
    assert_eq!(range.path, "src/lib.rs");
    assert_eq!(range.line_start, Some(2331));
    assert_eq!(range.line_end, Some(2334));

    let line = bachuc_timeline::links::code_citation("src/lib.rs:12").unwrap();
    assert_eq!(line.line_start, Some(12));
    assert_eq!(line.line_end, None);

    for meta in [
        "title=\"x\"",
        "title=\"a:1\"",
        "src/lib.rs line 5",
        "src/lib.rs(12)",
        "src/lib.rs:10 title=\"x\"",
        "src/lib.rs:12:4",
        "src/lib.rs",
        "",
        "  ",
    ] {
        assert_eq!(
            bachuc_timeline::links::code_citation(meta),
            None,
            "{meta:?}"
        );
    }
}
