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

## rust-skills

Source: https://github.com/leonardomso/rust-skills
Commit: `fd2a861ab0406a4ac536a55274d14ea6fd1ca9c9` (2026-06-14)
Installed: 2026-10-27 via `npx add-skill leonardomso/rust-skills`, then relocated from
`.agents/skills/rust-skills` (add-skill's universal layout) to `.omp/skills/rust-skills`.

Local adaptations: dropped `README.md`, `CLAUDE.md`, `AGENTS.md`, `CONTRIBUTING.md`,
`CHANGELOG.md`, `.github/` (foreign-harness docs, would be misread as agent context);
frontmatter invocation switched from `/rust-skills` slash-command to `skill://rust-skills`.
`rules/` (265 rule files) and `checks/` (corpus validator) kept verbatim — upstream prose is
the source of truth.

Update: `npx add-skill leonardomso/rust-skills`, then re-apply the deletions above and the
frontmatter line, diffing against the installed copy first.
