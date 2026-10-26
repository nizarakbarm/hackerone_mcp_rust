---
name: handoff
description: Compact the current conversation into a handoff document for another agent to pick up.
disable-model-invocation: true
---

Write a handoff document summarising the current conversation so a fresh agent can continue the work, then `write` it to the temporary directory of the user's OS (`$TMPDIR`, else `/tmp`; `%TEMP%` on Windows) — not the current workspace. Reply with the file path plus a ≤3-line summary; the document is the artifact, not the chat reply.

Include a "suggested skills" section in the document, naming each skill the next agent should fetch as `read skill://<name>` (the next agent starts blank; it must read them, never guess).

Do not duplicate content already captured in other artifacts (specs, plans, ADRs, issues, commits, diffs). Reference them by path or URL instead.

Redact any sensitive information, such as API keys, passwords, or personally identifiable information.

If the user passed arguments, treat them as a description of what the next session will focus on and tailor the doc accordingly.
