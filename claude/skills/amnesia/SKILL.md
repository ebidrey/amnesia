---
name: amnesia
description: >-
  Recover and persist agent memory as observations.
  Use to load recent context, search past discoveries, or save new observations across sessions and agents.
argument-hint: [recent -n N | search "topic" | save --type <type> | get <id>]
---

## Triggers

- Session start — load recent observations for continuity
- Architectural or design decision made
- Bug found (fixed or not)
- Non-obvious codebase discovery
- Reusable pattern established
- Unexpected error, failure, or workaround applied
- Approach changed mid-task
- User corrects or gives feedback
- Task completed — save summary

---

## Read your own recent context

```bash
amnesia recent --agent "<your-role>" -n 10
```

Flags:

- `--agent <name>` — your agent role name
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

Close each session with one `--type summary` observation covering the full session arc.

