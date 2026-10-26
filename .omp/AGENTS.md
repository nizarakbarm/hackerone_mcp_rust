# AGENTS.md

## Agent skills

### Issue tracker

GitHub Issues via the `gh` CLI (repo `nizarakbarm/hackerone_mcp_rust`); PRs are not a triage surface. See `docs/agents/issue-tracker.md`.

### Triage labels

Default five strings: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: root `GLOSSARY.md` + `docs/adr/`, created lazily by `skill://domain-modeling`. See `docs/agents/domain.md`.

## Engineering skills

Installed project-local in `.omp/skills/` (25 skills, upstream `mattpocock/skills` — see `.omp/skills/UPSTREAM.md`). Conventions: `.omp/RULES.md`.
