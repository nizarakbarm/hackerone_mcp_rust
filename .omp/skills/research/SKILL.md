---
name: research
description: Investigate a question against high-trust primary sources and capture the findings as a Markdown file in the repo. Use when the user wants a topic researched, docs or API facts gathered, or reading legwork delegated to a background agent.
---

# Research

Spin up a **background `task` entry** to do the research, so you keep working while it reads: one entry in a `task` call (`agent: task`, `isolated: true` if it may touch the repo; `agent: scout` if it only reads). Children start blank, so the brief is self-contained. Then get on with your own work; the result arrives when it lands.

Its job:

1. Investigate the question against **primary sources** (official docs, source code, specs, first-party APIs), not a secondary write-up of them. Follow every claim back to the source that owns it.
2. Write the findings to a single Markdown file, citing each claim's source.
3. Save it where the repo already keeps such notes; match the existing convention, and if there is none, put it somewhere sensible and say where.

When it lands, **reply with that file's path plus a ≤3-line summary**; don't paste the findings into chat.
