# TASK-103 Live check of running and waiting rows

Group: H (lead-run check; writes only its outcome)
Class: docs

## Brief

Goal: See the running row (spinner, time against average) and the amber waiting row (Open thread) on the live page, which the first live check could not show.

Change: These two row states are UNVERIFIED -> they are seen on live, with screenshots and notes.

Boundaries:

- Use a fixture outside the repo: `/tmp/wt-fixture` with `watchtower/NEXT.md` holding one `DONE`, one `IN PROGRESS`, and one `TODO` TASK, and a git history with one commit per done TASK.
- Agents on live only, in the fixture's workspace (`paseo run --workspace <id>`): a lead, a builder labelled with the IN PROGRESS TASK that is running, and a builder that waits for a permission (Claude Haiku in a mode that asks, prompted to run a shell command). Never on release.
- Archive the agents after the check. Leave the fixture in `/tmp`.
- A wrong render is a finding for the owner, not a code change in this TASK.

How:

- Build the fixture, start the agents, open the page with CDP on port 9223, and take screenshots.

Files:

- [watchtower/tasks/TASK-103-outcome.md](watchtower/tasks/TASK-103-outcome.md) (results)

Expected result:

- Screenshot of a running row with a spinner and "Nm / Nm".
- Screenshot of an amber waiting row with Open thread that opens the builder thread.

Anti-goal: Nothing reaches release. Tripwire: every agent this TASK creates lists on live (`npm run cli -- ls`) and none on release; read at the end.

## Verify

- Needs a human or a browser session: the two screenshots exist and match the Expected result.
- Anti-goal: `npm run cli -- ls -g` lists the test agents as archived; `PASEO_HOME=~/.bachuc paseo ls -g` lists none of them.
