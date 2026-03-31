# amnesia

> Your AI agent has amnesia. This search engine fixes it.
> Writable and searchable persistent memory for AI agents


<img width="1000" height="701" alt="Screenshot 2026-03-09 at 8 23 29 AM" src="https://github.com/user-attachments/assets/0ab9c2de-f33d-48fb-b364-c6593b546b7f" />
Goldfish: Often cited for having very poor, short-term memory.

---

## What is this?

Every time you start a new session with an AI coding agent, it starts completely blank. It has no idea what decisions were made last week, what bugs were fixed yesterday, or what patterns the team agreed on. It will rediscover the same things, make the same mistakes, and ask the same questions - over and over.

`amnesia` is a small command-line tool that gives agents a place to write down what they learn and look it up later. It is a single binary, writes to plain text files, and requires no running process. Agents call it through the shell, the same way they run `git` or `grep`.

That is the whole thing.

---

## The unit of memory: an observation

Everything stored in `amnesia` is an **observation** - a structured note that captures one meaningful event from a session.

```
id:        01JNAAAA0000000000000000AA        unique ID (ULID, time-sortable)
timestamp: 2026-03-07T14:23:01Z             when it was saved
agent:     backend-developer                who wrote it
type:      bugfix                           what kind of event it was
title:     JWT expiry check used local time instead of UTC
content:   chrono::Local::now() was compared against the exp claim which is
           always UTC. Fixed with Utc::now(). Tokens expired 5h early in
           UTC-5 environments.
files:     src/auth/jwt.rs                  files involved (optional)
tags:      auth, jwt, timezone              free-form tags (optional)
session_id: 01KK7V16Q9V8NMSYP7JZS1F2BX      session that produced it (optional)
```

`search` and `recent` show a compact view - id, agent, type, timestamp, and title. The `content` field is intentionally omitted to keep token usage low. Use `amnesia get <id>` to read the full content of any observation.

An observation has six types, each with a clear meaning:

| Type | Meaning |
|------|---------|
| `decision` | An architectural or design decision was made |
| `bugfix` | A bug was found and fixed |
| `discovery` | Something non-obvious was learned about the codebase |
| `pattern` | A reusable pattern was established |
| `warning` | Something other agents should avoid |
| `summary` | End-of-session summary |

That's the data model. One observation per event, one line in a file.

---

## Philosophy

**Simple by design.**

- One binary. No daemon, no server, no MCP, no database engine.
- Plain text files: NDJSON, one observation per line. Open it in any editor.
- Agents call it via shell. Zero framework overhead, zero extra context tokens consumed.
- BM25 full-text search built in. No embeddings, no vector store, no API key needed.
- Human-readable storage. You can `grep` it, `cat` it, back it up with `cp`, inspect it with `jq`.

The goal is a tool so simple that there is no reason not to use it. Every feature that adds complexity is a reason for an agent (or a developer) to skip it.

---

## Install

### From source (requires Rust)

```bash
git clone https://github.com/ebidrey/amnesia
cd amnesia
cargo install --path .
```

This installs the binary to `~/.cargo/bin/amnesia`.

If `amnesia` is not found after install, add Cargo bin to your PATH:

```bash
export PATH="$HOME/.cargo/bin:$PATH"
```

### Verify

```bash
amnesia --version
amnesia stats
```

Tested on 2026-03-31 in this repository with:

```bash
cargo install --path .
~/.cargo/bin/amnesia --version
~/.cargo/bin/amnesia stats
```

The store file is created automatically on the first `amnesia save`.

---

## Launcher

Run `amnesia` with no subcommand to open the interactive TUI launcher:

```bash
amnesia
```

It prompts you to select a **project** (from `~/.amnesia/projects.toml`) and an **orchestrator** (Claude, OpenCode, Cursor, Aider, Goose), then launches the orchestrator with `AMNESIA_PROJECT` and `AMNESIA_SESSION` set in the environment.

From that point on, every `amnesia save` inside that orchestrator session is automatically scoped to the project and tagged with the session ID - no extra flags required.

---

## Project-scoped storage

When `AMNESIA_PROJECT` is set, all commands read and write to a project-specific store instead of the global one:

```
~/.amnesia/projects/<name>/store.ndjson
```

The global store (`~/.amnesia/store.ndjson`) is still used when `AMNESIA_PROJECT` is not set.

You can select a project in three ways, in order of precedence:

```bash
# 1. --project flag (any subcommand)
amnesia search --project myproject "authentication"
amnesia --project myproject recent -n 5
amnesia --project myproject sessions

# 2. AMNESIA_PROJECT environment variable
AMNESIA_PROJECT=myproject amnesia search "authentication"

# 3. The launcher sets it automatically before starting the orchestrator
amnesia
```

`--project` takes precedence over `AMNESIA_PROJECT` when both are present.

---

## Commands

### `save` - write an observation

```bash
amnesia save \
  --agent "backend-developer" \
  --type bugfix \
  --title "JWT expiry check was using local time instead of UTC" \
  --content "chrono::Local::now() was compared against the exp claim which is always UTC. Fixed by using Utc::now() everywhere. Tokens were expiring 5 hours early in UTC-5 environments." \
  --files "src/auth/jwt.rs" \
  --tags "auth,jwt,timezone"
```

Output:
```
saved 01JNAAAA0000000000000000AA
```

`--files` and `--tags` are optional. `--agent`, `--type`, `--title`, and `--content` are required.

**Session tagging:** if `AMNESIA_SESSION` is set in the environment, it is automatically attached to the observation as `session_id`. You can also pass `--session <id>` explicitly:

```bash
amnesia save --agent "backend-developer" --type bugfix --title "..." --content "..." \
  --session 01KK7V16Q9V8NMSYP7JZS1F2BX
```

The launcher sets `AMNESIA_SESSION` automatically, so you normally do not need to pass it by hand.

---

### `search` - find relevant observations

Full-text search across titles, content, and tags using BM25 ranking. Omit the query to get the newest observations first.

```bash
# search by topic
amnesia search "JWT authentication"

# filter by agent and type
amnesia search "pagination" --agent backend-developer --type bugfix

# filter by date range
amnesia search "deployment" --after 2026-01-01 --before 2026-03-01

# filter by affected file
amnesia search --files "src/auth"

# filter by session
amnesia search --session 01KK7V16Q9V8NMSYP7JZS1F2BX

# return newest, no query needed
amnesia search
```

Output is compact - titles only, no content:
```
id:        01JNAAAA0000000000000000AA
agent:     backend-developer
type:      bugfix
timestamp: 2026-03-07T14:23:01Z
title:     JWT expiry check was using local time instead of UTC

id:        01JN99990000000000000000ZZ
agent:     api-designer
type:      decision
timestamp: 2026-03-06T10:00:00Z
title:     All list endpoints return envelope with meta
```

This is intentional. Showing full content for every result would consume tokens on observations you may not care about. The typical workflow is:

```bash
amnesia search "jwt"        # scan titles, find candidates
amnesia get 01JNAAAA        # read full content only for the relevant ones
```

---

### `get` - read a full observation

```bash
amnesia get 01JNAAAA
```

Any unambiguous prefix of the ULID works.

Output:
```
id:        01JNAAAA0000000000000000AA
timestamp: 2026-03-07T14:23:01Z
agent:     backend-developer
type:      bugfix
title:     JWT expiry check was using local time instead of UTC
content:   chrono::Local::now() was compared against the exp claim which is
           always UTC. Fixed by using Utc::now() everywhere. Tokens were
           expiring 5 hours early in UTC-5 environments.
files:     src/auth/jwt.rs
tags:      auth, jwt, timezone
```

---

### `recent` - last N observations

Same compact format as `search` - titles only. Use `amnesia get <id>` for full content.

```bash
# last 10 across all agents
amnesia recent

# last 5 from a specific agent
amnesia recent --agent backend-developer -n 5

# last 10 from a specific session
amnesia recent --session 01KK7V16Q9V8NMSYP7JZS1F2BX
```

---

### `sessions` - list recent sessions

Requires `AMNESIA_PROJECT` to be set.

```bash
AMNESIA_PROJECT=myproject amnesia sessions
AMNESIA_PROJECT=myproject amnesia sessions -n 5
```

Output:
```
id:           01KK7V16Q9V8NMSYP7JZS1F2BX
project:      myproject
orchestrator: claude
started_at:   2026-03-08T22:05:00Z
```

Sessions are stored in `~/.amnesia/projects/<name>/sessions.ndjson`, separate from observations.

---

### `projects` - list all projects

```bash
amnesia projects
```

Output:
```
project: amnesia
store:   /Users/you/.amnesia/projects/amnesia/store.ndjson
obs:     42

project: philosophy
store:   /Users/you/.amnesia/projects/philosophy/store.ndjson
obs:     0
```

Projects are listed alphabetically. Each entry shows the project name, the full path to its `store.ndjson`, and the number of observations it contains. If no projects have been created yet, prints `no projects found`.

---

### `stats` - store overview

```bash
amnesia stats
```

```
total:    47 observations
agents:   backend-developer (23), api-designer (12), orchestrator (12)
types:    decision (18), bugfix (14), discovery (9), pattern (4), warning (2)
oldest:   2026-01-15
newest:   2026-03-07
file:     ~/.amnesia/store.ndjson (84 KB)
```

---

## Integrating with Claude

The real value comes from teaching your AI agent to use `amnesia` consistently. Add these instructions to your `~/.claude/CLAUDE.md` file:

```markdown
## Memory

You have access to `amnesia` - a persistent memory CLI for storing and
retrieving observations across sessions.

**At the start of every session:** run `amnesia recent -n 10` to recover
context from previous sessions. If the task touches a specific area, also
run `amnesia search "<topic>"` before starting.

**After significant work:** save an observation with `amnesia save`.
Significant work includes: fixing a bug, making an architectural decision,
discovering something non-obvious, establishing a pattern, or ending a session.

**Before starting a task:** run `amnesia search "<topic>"` to check if past
sessions left relevant context.

**Never skip saving** after a session with meaningful output.
```

Or install the included Claude Code skill:

```bash
cp -r skills/amnesia ~/.claude/skills/amnesia
```

### What agents should save

**After fixing a bug:**
```bash
amnesia save \
  --agent "backend-developer" \
  --type bugfix \
  --title "Connection pool exhaustion under high load" \
  --content "Default pool size was 10. Under load tests, all connections were held by slow queries in the reporting module. Increased pool to 50 and added query timeout of 5s." \
  --files "src/db.rs,src/config.rs" \
  --tags "postgresql,connection-pool,performance"
```

**After an architecture decision:**
```bash
amnesia save \
  --agent "api-designer" \
  --type decision \
  --title "All list endpoints return envelope with meta" \
  --content "Response shape: {data: [...], meta: {total, page, per_page, next_cursor}}. Cursor-based pagination for performance. Avoids page drift on live data. All list endpoints must conform to this shape." \
  --files "src/dto/pagination.rs,docs/api-conventions.md" \
  --tags "api,pagination,conventions"
```

**After discovering something non-obvious:**
```bash
amnesia save \
  --agent "backend-developer" \
  --type discovery \
  --title "PostgreSQL jsonb operators are not indexed by default" \
  --content "Queries on jsonb fields with ->> operator do a full table scan unless a GIN index is added. Added GIN index on metadata column. Query time dropped from 800ms to 4ms." \
  --files "migrations/20260210_add_gin_index.sql" \
  --tags "postgresql,jsonb,indexing,performance"
```

**End of session:**
```bash
amnesia save \
  --agent "backend-developer" \
  --type summary \
  --title "Session: auth module refactor complete" \
  --content "Replaced custom session tokens with JWT. Added refresh token rotation. All token operations go through auth::token module. 47 tests passing. Breaking change: clients must handle 401 with WWW-Authenticate header." \
  --files "src/auth/mod.rs,src/auth/jwt.rs,src/middleware/auth.rs" \
  --tags "auth,jwt,refactor"
```

### Using multiple agents

`amnesia` is designed for multi-agent workflows. Each agent identifies itself with `--agent`. A naming convention like role names works well: `backend-developer`, `api-designer`, `frontend-developer`, `orchestrator`.

Agents can search across all agents (no filter) or scope to a specific one:

```bash
# what did the api-designer decide about authentication?
amnesia search "authentication" --agent api-designer --type decision

# what warnings have been issued across all agents?
amnesia search --type warning

# what touched the auth module?
amnesia search --files "src/auth"

# what happened in this session?
amnesia search --session 01KK7V16Q9V8NMSYP7JZS1F2BX
```

---

## Configuration

Optional config at `~/.amnesia/config.toml`. All values have defaults - the file does not need to exist.

```toml
store_path    = "~/.amnesia/store.ndjson"
default_limit = 10
```

---

## Storage format

### Observations

Stored as NDJSON - one JSON object per line:

```json
{"id":"01JNAAAA0000000000000000AA","timestamp":"2026-03-07T14:23:01Z","agent":"backend-developer","op_type":"Bugfix","title":"JWT expiry check used local time instead of UTC","content":"...","files":["src/auth/jwt.rs"],"tags":["auth","jwt","timezone"]}
```

`session_id` is included only when the observation was saved with a session:

```json
{"id":"01KK7VFQ...","timestamp":"2026-03-08T23:10:59Z","agent":"backend-developer","op_type":"Bugfix","title":"...","content":"...","files":[],"tags":[],"session_id":"01KK7V16Q9V8NMSYP7JZS1F2BX"}
```

### Sessions

Stored separately in `~/.amnesia/projects/<name>/sessions.ndjson`:

```json
{"id":"01KK7V16Q9V8NMSYP7JZS1F2BX","project":"myproject","orchestrator":"claude","started_at":"2026-03-08T22:05:00Z"}
```

### Properties

- IDs are [ULIDs](https://github.com/ulid/spec) - lexicographically sortable, unique, collision-free.
- Append-only. `save` only adds a line, never rewrites the file.
- Append is atomic on most filesystems - safe for concurrent writes from parallel agents.
- No automatic pruning. The file grows over time. Back it up, inspect it, `grep` it - it is just text.

---

## How search works

Search uses [BM25](https://en.wikipedia.org/wiki/Okapi_BM25) ranking, built in-memory at query time. No persistent index. The store is read once per invocation and indexed on the fly. Fast enough for thousands of observations and trivially simple to maintain.

Field weights:

| Field | Weight |
|-------|--------|
| title | 2.0 |
| tags | 1.5 |
| content | 1.0 |

When no query is given, results are returned newest-first by timestamp.

---

## Contributing

The codebase is intentionally small. Every module has inline `#[cfg(test)]` unit tests.

```
src/
  main.rs          CLI entry point (clap)
  model.rs         Observation + Session structs, OpType enum
  store.rs         NDJSON read / append for observations (path-parametric)
  sessions.rs      NDJSON read / append for sessions
  bm25.rs          BM25 implementation (~100 lines)
  filter.rs        agent / type / date / files / session filters
  config.rs        config.toml loading, project path helpers
  launcher.rs      TUI launcher: project + orchestrator selector
  projects.rs      projects.toml loading
  commands/
    save.rs
    search.rs
    get.rs
    recent.rs
    projects.rs
    sessions.rs
    stats.rs
```

```bash
cargo test
cargo build --release
```

---

## License

MIT
