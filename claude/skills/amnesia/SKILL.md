---
name: amnesia
description: >-
  Mandatory memory skill — INVOKE AT SESSION START before anything else.
  Use when saving decisions, discoveries, bugfixes, warnings, patterns, or summaries;
  loading recent context; or searching past observations across sessions and agents.
argument-hint: [recent -n N | search "topic" | save --type <type> | get <id>]
---

## When to invoke this skill

Invoke `/amnesia` at **session start** — always, as the very first action, before reading
files or answering.

Mid-session, do NOT re-invoke the skill. Just run `amnesia save ...` directly whenever
any of the events in the triggers table below occur.

---

## Session start

Run this immediately after loading the skill:

```bash
amnesia recent -n 10
```

This loads the last 10 observations across all agents, giving you continuity from prior
sessions. Omit `--agent` here — cross-agent context is more valuable than filtering.

---

## Agent naming

Use `--agent` ONLY when saving, never when reading (`recent` / `search`).

- Main agent: `--agent "main"`
- Subagents: use the agent type they were spawned as — `"general-purpose"`, `"Explore"`,
  `"Plan"`, etc. This name is fixed by the system, not chosen per task.

The parent agent MUST tell each subagent its agent name in the prompt.

---

## Mid-session triggers — save IMMEDIATELY

Do NOT batch. Do NOT wait. Save THE MOMENT the event occurs, **before continuing
to the next step**. This means: if you just made a decision, save it NOW, before
writing the next line of code. If a task just finished, save the summary NOW,
before starting the next task. Saving is not a cleanup step — it is part of the work.

| Event | Type |
|-------|------|
| Architectural or design choice made | decision |
| User makes a design/scope decision or rejects an approach | decision |
| Bug found and left unsolved | warning |
| Bug found and fixed | bugfix |
| Non-obvious codebase discovery | discovery |
| Reusable approach or convention established | pattern |
| Unexpected error, failure, or workaround applied | warning |
| Dependency or version constraint discovered | discovery |
| Approach changed mid-task (by you or by user input) | decision |
| User corrects agent behavior or gives a technical preference | pattern |
| Read a file and found something surprising or counterintuitive | discovery |
| Tried something and it failed for a non-obvious reason | warning |
| Identified a potential risk or tech debt | warning |
| **Task or unit of work completed (see below)** | **summary** |

### Task completion summaries

A "task" is any discrete unit of work: a feature, a bugfix, a refactor, a
migration, a review — anything that has a clear start and end. If you are
working through a list of subtasks, each completed subtask gets its own summary.

Save a `--type summary` after completing **every** task — no exceptions. Even if
mid-session saves were already made for individual events, the summary is still
required. Cover: what was done, why, and what state things were left in.

The agent CANNOT detect session end (Ctrl+D kills the process instantly). Task
summaries are the only safety net — do not skip them.

### Save vs. skip — concrete examples

```
✅ Save: tried an approach, it failed for a non-obvious reason
✅ Save: read a file and found an unexpected constraint or behavior
✅ Save: made a tradeoff between two valid options
✅ Save: user corrected a technical preference or approach
✅ Save: user decided to exclude something from scope
✅ Save: finished implementing a subtask (summary)

❌ Skip: read a file with no surprises
❌ Skip: ran a command that worked as expected
❌ Skip: wrote code following an already-established pattern
```

### What NOT to save

**Only save observations about events that ACTUALLY HAPPENED in this conversation.**
Never re-save or paraphrase from prior context (CLAUDE.md, amnesia output). Those are
reference material, not current events.

---

## Subagents

Subagents see this skill in their context automatically and will invoke `/amnesia`
at their start. The parent agent only needs to tell the subagent its agent name
so it can use `--agent` correctly when saving (see Agent naming above).

Before starting work, the subagent MUST run:

```bash
amnesia search "<domain keywords from delegated task>" --limit 5
```

If a subagent lacks Bash access, the parent agent MUST save the relevant observations
on their behalf immediately after receiving the subagent's results.

---

## Read your own recent context

```bash
amnesia recent -n 10
```

Flags:

- `--agent <name>` — filter to one agent (omit for cross-agent context)
- `-n <number>` — number of observations to return (default: 10)

Output — one block per observation, blank line between each:

```
id:        <ulid>
agent:     <agent>
type:      <type>
timestamp: <YYYY-MM-DDTHH:MM:SSZ>
title:     <title>
files:     <file1>, <file2>
```

To read the full content of a specific observation:

```bash
amnesia get <id-prefix>
```

Output:

```
id:        <ulid>
timestamp: <YYYY-MM-DDTHH:MM:SSZ>
agent:     <agent>
type:      <type>
title:     <title>
content:   <text, multiline continuation indented>
files:     <file1>
           <file2>
tags:      <tag1>, <tag2>
```

---

## Search relevant context across all agents

```bash
amnesia search "<topic>"
amnesia search "<topic>" --agent "<name>" --type <type> --after <YYYY-MM-DD> --before <YYYY-MM-DD> --files "<path-substring>" --limit <n>
```

Flags:

- `<topic>` — optional free-text, BM25 ranked across title + content + tags
- `--agent <name>` — restrict to one agent
- `--type <type>` — restrict to one type (see enum below)
- `--after <YYYY-MM-DD>` — on or after this date
- `--before <YYYY-MM-DD>` — on or before this date
- `--files <substring>` — substring match on any file path in the observation
- `--limit <n>` — max results (default: 10)

Output: same format as `recent`. Use `amnesia get <id-prefix>` to read full content.

If nothing matches: `no results`

---

## Save each significant item from this session

```bash
amnesia save \
  --agent "<your-role>" \
  --type <type> \
  --title "<one-line summary, max ~80 chars>" \
  --content "<what happened, why, how it was resolved, where in the codebase>" \
  --files "<comma-separated relative file paths>" \
  --tags "<comma-separated keywords>"
```

Required: `--agent`, `--type`, `--title`, `--content`
Optional: `--files`, `--tags`

### Writing quality guidelines

**ALL content (title, content, tags) MUST be written in English, regardless of
the conversation language.**

Search uses BM25 ranking across `title + content + tags`. Write with
searchability in mind.

- **`--title`**: Max ~80 chars. Specific and concise — like a commit message.
  - Bad: `"found something interesting about the config"`
  - Good: `"redis config uses lazy-free on eviction, not default"`
- **`--content`**: Full explanation. Include what happened, why, how it was
  resolved (if applicable), and where in the codebase. All detail goes here,
  not in the title.
- **`--tags`**: Short, lowercase keywords for filtering. Use domain terms, file
  names, tech names. 3-6 tags is ideal.
  - Bad: `"interesting finding about configuration"`
  - Good: `"redis, config, eviction, lazy-free, performance"`
- **`--files`**: Always include affected file paths when applicable.

Valid values for `--type` (enum — only these are accepted):

| type      | use when                                           |
|-----------|----------------------------------------------------|
| decision  | an architectural or design decision was made       |
| bugfix    | a bug was found and fixed                          |
| discovery | something important was learned about the codebase |
| pattern   | a reusable pattern was established                 |
| warning   | something other agents should avoid                |
| summary   | end-of-session summary                             |

Output on success: `saved <ulid>`
