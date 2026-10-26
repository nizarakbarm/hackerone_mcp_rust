# Upstream provenance

Source: https://github.com/mattpocock/skills
Commit: `5beb35b040e3de97e54fa9e386d7e42d15c295c2` (2026-10-19)
Taken from: `skills/engineering/*` (all) + `skills/productivity/{grill-me,grilling,handoff,writing-for-agents}`
Installed: 2026-10-24, project-local (`.omp/skills/`). Not installed globally.

Local adaptations: native omp tool names, `skill://` cross-references, SKILL.md size caps with
`references/` for detail, foreign-harness mechanics removed. See `AGENTS.md` for conventions.

Update:
```sh
git clone --depth 1 https://github.com/mattpocock/skills /tmp/mattskills
# re-diff a skill before copying: upstream prose is the source of truth
diff -u .omp/skills/<name>/SKILL.md /tmp/mattskills/skills/<group>/<name>/SKILL.md
``
