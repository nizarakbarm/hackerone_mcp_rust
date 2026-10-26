# Scoping the procedure

Step 1 reads the repo before asking anything cold.

## For setup work

- `.env`, `.env.example`, `.env.*`
- `README`
- `docker-compose*` and framework config
- `.github/workflows/*` — every `secrets.*` / `vars.*` reference is a value the wizard must produce

## For a migration or transition

- The current state, the target state, and the irreversible actions between them.

## What step 1 must end with

Every stage named in order, and for each captured value:

- (a) where the human gets it
- (b) where it's written (`.env`, a GitHub secret, both, or nowhere — some stages are pure actions)
- (c) whether it's secret (hidden entry) or public

Show the user that ordered list of stages and the values each produces, and confirm: they may add, drop, or reorder.
