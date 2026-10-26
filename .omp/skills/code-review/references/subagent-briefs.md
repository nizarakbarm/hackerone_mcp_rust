# Sub-agent briefs

Both entries in the `task` call are `agent: reviewer`. Each brief must be self-contained — the child sees nothing from this session. Paste the shared material into both.

## Standards entry

Its brief should include:

- The full diff command and commit list.
- The list of standards-source files you found in step 3, **plus the smell baseline from `skill://code-review/references/smells.md`** pasted in full (the sub-agent has no other access to it).
- The brief: "Report, per file/hunk where relevant, (a) every place the diff violates a documented standard: cite the standard (file + the rule); and (b) any baseline smell you spot: name it and quote the hunk. Distinguish hard violations from judgement calls: documented-standard breaches can be hard, but baseline smells are always judgement calls, and a documented repo standard overrides the baseline. Skip anything tooling enforces. Under 400 words."

## Spec entry

Its brief should include:

- The diff command and commit list.
- The path or fetched contents of the spec.
- The brief: "Report: (a) requirements the spec asked for that are missing or partial; (b) behaviour in the diff that wasn't asked for (scope creep); (c) requirements that look implemented but where the implementation looks wrong. Quote the spec line for each finding. Under 400 words."

If the spec is missing, skip the Spec entry and note this in the final report.

Return each report as a file with its path, or inline if short.
