Full amnesia workflow: recover context, search across agents, and save observations.

---

## Read your own recent context

```bash
amnesia recent --agent "<your-role>" -n 10
```

Flags:

- `--agent <name>` — your agent role name
- `-n <number>` — number of observations to return (default: 10)
- `--session <id>` — restrict to a specific session (uses `AMNESIA_SESSION` if set)

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
amnesia search "<topic>" --agent "<name>" --type <type> --after <YYYY-MM-DD> --before <YYYY-MM-DD> --files "<path-substring>" --limit <n> --session <id>
```

Flags:

- `<topic>` — optional free-text, BM25 ranked across title + content + tags
- `--agent <name>` — restrict to one agent
- `--type <type>` — restrict to one type (see enum below)
- `--after <YYYY-MM-DD>` — on or after this date
- `--before <YYYY-MM-DD>` — on or before this date
- `--files <substring>` — substring match on any file path in the observation
- `--limit <n>` — max results (default: 10)
- `--session <id>` — restrict to a specific session

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
Optional: `--files`, `--tags`, `--session <id>`

If the environment variable `AMNESIA_SESSION` is set, its value is automatically attached to every saved observation as `session_id`. You can also pass `--session <id>` explicitly to override it.

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
