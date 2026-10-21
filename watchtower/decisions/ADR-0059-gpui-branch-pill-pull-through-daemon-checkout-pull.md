# ADR-0059 GPUI branch pill Pull through the daemon checkout pull over a plugin-server pull

Status: accepted
Date: 2026-10-21
Supersedes: -
Scope: `packages/desktop-gpui/plugins/thread-branch/src/**`, `hiep-plugins/plugins/thread-branch/server/git.ts`

## Context

- The branch pill menu has Fetch and Refresh, which go through the thread-branch server RPC `thread-branch.get` ([git.ts](hiep-plugins/plugins/thread-branch/server/git.ts) `read`).
- The daemon already serves `checkout_pull_request`. It runs `git pull`, aborts a conflict state on failure, and tells other views that git changed ([checkout-session.ts](packages/server/src/server/session/checkout/checkout-session.ts) `handleCheckoutPullRequest`).
- The GPUI explorer uses that request through `Client::git_action` with `GitAction::Pull`, and blocks a pull while the folder has local changes ([git_actions.rs](packages/desktop-gpui/crates/explorer/src/git_actions.rs) `pull_unavailable`).
- Hiep's git config sets no `pull.rebase` or `pull.ff`, so a diverged branch makes `git pull` fail instead of creating a merge commit. Checked on 2026-10-21.

## Decision

- The branch pill Pull row calls `Client::git_action(cwd, GitAction::Pull, cx)`. It adds no thread-branch server code.
- Like the explorer, Pull is blocked while the folder has local changes. The check uses a fresh `Force` read, not the cached pill info.
- Hiep chose this on 2026-10-21, in the `/watchtower new` interview.

## Consequences

- The pill Pull and the explorer Pull behave the same way, with the same conflict abort and view refresh.
- Only the GPUI crate changes, so no plugin reload is needed in release or live.
- If a machine sets `pull.rebase` or `pull.ff=false`, Pull follows that config.

## Options Considered

- A `pull` mode in `thread-branch.get` that runs `git pull --ff-only` in the plugin server: rejected. It duplicates the daemon pull, misses its conflict abort and git-change notice, and needs a server change and a plugin reload.
- Let git decide on a dirty folder: rejected. An agent may be editing files in that folder, and the explorer already blocks this case.

## Revisit If

- Hiep wants Pull to stay fast-forward only on every machine, whatever the git config says.
- The daemon pull changes behavior, for example it starts to stash local changes.
