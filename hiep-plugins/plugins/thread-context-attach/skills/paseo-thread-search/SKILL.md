---
name: paseo-thread-search
description: Use when you need a decision, result, root cause, commit, or other context from another Paseo thread or agent conversation, for example "what did the other thread find about X". Searches exported Paseo threads with qmd and reads only the matching turn. Do NOT use for source-code search, for the personal knowledge base (use kb-search), or for web search.
---

# Paseo Thread Search

## Overview

The `thread-context-attach` Paseo plugin exports every Paseo thread to markdown and indexes it in the qmd index `paseo-threads`. Each file is one thread. Each `## Turn N` section holds one user message (`### Asked`) and the assistant replies to it (`### Answer`). Tool calls and tool output are not included.

Files live in `~/.paseo/plugin-data/thread-context-attach/threads/<agentId>.md`. The index updates about 30 seconds after a thread finishes a turn.

## How to Use

Always pass `--index paseo-threads`.

1. Search with 2 or 3 distinctive keywords. Every word must appear in the thread, so do not write a full sentence.

   ```bash
   qmd --index paseo-threads search "watchtower sidebar" -n 5
   ```

   Each hit prints `qmd://paseo-threads/<agentId>.md:<line>`, the thread title, a score, and a short snippet.

   Replace every `.` with a space. One keyword with a dot, such as `agents.list` or `0.9.1`, makes the whole search return nothing; `agents list` and `0 9 1` work. Hyphens are fine.

2. If the keywords miss, retry with synonyms or other names for the same thing. Threads are often in Vietnamese, so also retry in the other language: `Jev phân loại` finds a thread that `Jev classifier` misses. Vietnamese diacritics are optional (`phan loai` matches too). Then try the hybrid search with a short question. It usually takes 1 to 3 seconds:

   ```bash
   qmd --index paseo-threads query "why does the git pill disappear" --no-rerank -n 5
   ```

   Keep `--no-rerank`: reranking made queries take 30 to 40 seconds at the median and put the right thread first at most once more in 22 test questions. `query` always returns hits, even when nothing matches, so check each snippet before you use it. The right thread is often second or third.

3. Read only the part you need, starting a little above the hit line:

   ```bash
   qmd --index paseo-threads get "qmd://paseo-threads/<agentId>.md:<line>" -l 80
   ```

   To see where each turn starts, list the turn headings:

   ```bash
   rg -n "^## Turn" ~/.paseo/plugin-data/thread-context-attach/threads/<agentId>.md
   ```

Do not read a whole thread file. A long thread can be 10,000 tokens or more.

## Rules

- Cite what you use as `thread <first 8 chars of agentId>, Turn N`.
- Results can include the current conversation. Skip hits that repeat what you already know from this thread.
- A thread reply is what an agent said, not proof. Check important claims against code, git, or tests.
- If 3 different keyword sets miss and no `query` snippet is about the topic, say that nothing was found. Do not guess.
- If `qmd` reports no collection or no files, the plugin is not installed or has not run yet. Say so.
