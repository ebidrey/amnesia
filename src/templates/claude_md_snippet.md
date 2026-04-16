## Memory — NON NEGOTIABLE
Use the `/amnesia` skill for all memory operations.
**At session start, invoke `/amnesia` before anything else.**
The skill contains the canonical workflow and must be followed.
When delegating to a subagent, always tell it its agent name (see Agent naming
in the skill) so it can use `--agent` correctly when saving.
### Proactive saving — ZERO TOLERANCE
Conversations can die at any time (crash, ctrl+D, network drop). If it wasn't
saved, it's gone. There is no "I'll save it later."
**After every user message that contains a decision, rejection, preference, or
design direction, your NEXT tool call must be an `amnesia save` operation —
before writing code, before reading files, before anything else.** This includes:
- User says "let's go with X" or "I prefer Y" → save decision immediately
- User rejects an approach → save decision immediately
- User defines scope ("only these files", "not that") → save decision immediately
- You and user agree on a plan or architecture → save decision immediately
- You discover something non-obvious → save discovery immediately
- A bug is fixed or a workaround applied → save immediately
Design conversations are the HIGHEST PRIORITY to save — they leave no artifact.
Code changes have git. Decisions only live in context, and context is fragile.
