# Spec source discovery

Look for the originating spec, in this order:

1. Issue references in the commit messages (`#123`, `Closes #45`, GitLab `!67`, etc.), fetched via the workflow in the tracker doc.
2. A path the user passed as an argument.
3. A spec file under `docs/`, `specs/`, or `.scratch/` matching the branch name or feature.
4. Nothing found → ask the user where the spec is. If they say there isn't one, the **Spec** axis skips and reports "no spec available".
