# Instructions


## Memory — NON NEGOTIABLE

Use the `/amnesia` skill for all memory operations. This is mandatory. If you
skip an amnesia save that this file requires, note the skip and reason in your
response so the user can audit compliance.

**Default rule: when in doubt, SAVE. It is always better to save too much than
too little. A missed save is a lost insight — an extra save costs nothing.**

### Agent naming

Use `--agent` ONLY when saving, never when reading (recent/search) — omitting
it returns observations from all agents, which gives better cross-agent
context.

- Main agent: always `--agent "main"`.
- Subagents: use the agent type they were spawned as — `"general-purpose"`,
  `"Explore"`, `"Plan"`, etc. This name is fixed by the system, not chosen
  per task, so it is deterministic and idempotent across sessions.

The parent agent MUST tell each subagent its agent name in the prompt.

### Session start (before anything else)

1. `amnesia recent -n 10`

### Mid-session triggers — MUST save IMMEDIATELY when any of these happen

Do NOT batch. Do NOT wait until the end. Save THE MOMENT the event occurs,
before continuing with the next step.

**CRITICAL: Only save observations about events that ACTUALLY HAPPENED in the
current conversation.** Never re-save, paraphrase, or infer events from
pre-loaded context (CLAUDE.md, memory files, amnesia recent output). Those are
reference material, not current events. If you haven't done it or seen it
happen in THIS session, don't save it.

| Event | Type | Examples |
|-------|------|----------|
| You make an architectural or design choice | decision | chose library X over Y, picked a DB schema approach, decided on an API shape |
| You find a bug and leave it unsolved | warning | found a race condition but fixing it is out of scope |
| You find a bug and fix it | bugfix | off-by-one, null ref, wrong import, broken config |
| You learn something non-obvious about the codebase | discovery | hidden dependency, undocumented env var, implicit coupling between modules |
| You establish a reusable approach or convention | pattern | error handling style, test structure, naming convention |
| You hit an unexpected error or failure | warning | build failure, flaky test, permission issue, unexpected API response |
| You apply a workaround or hack | warning | temporary fix, monkey-patch, TODO left behind |
| You discover a dependency or version constraint | discovery | package X requires Y >= 2.0, incompatible peer deps |
| You change your approach mid-task | decision | started with approach A, switched to B because of X |
| The user corrects you or gives feedback on your approach | pattern | user says "don't do X", "prefer Y over Z" |

### Additional proactive triggers

- You read a file and find something surprising or counterintuitive — **save as discovery**
- You try something and it fails for a non-obvious reason — **save as warning**
- You identify a potential risk or tech debt — **save as warning**

### Task completion summaries (replaces "session end")

The agent CANNOT detect when the user closes the session (Ctrl+D kills the
process instantly). Therefore, do NOT rely on a "session end" event.

Instead, MUST save a `--type summary` immediately after completing each
significant task. A task is significant if it involved any code change,
decision, discovery, bugfix, or multi-step reasoning. The summary should cover
what was done, why, and what state things were left in.

This ensures nothing is lost even if the user exits without warning.

### Subagents and amnesia

When delegating work to a subagent via the Agent tool, ALWAYS include the full
contents of this file (`.claude/CLAUDE.md`) in the subagent prompt so it
follows the exact same amnesia rules as the parent agent. The only difference:
subagents MUST use their agent type name for `--agent` when saving (see
"Agent naming" above).

If a subagent type does not have Bash access, the parent agent MUST save the
relevant observations on behalf of the subagent immediately after receiving its
results.

### Subagent context loading

Before starting work, the subagent MUST extract domain keywords from its
delegated task and run: `amnesia search "<keywords>" --limit 5`.
