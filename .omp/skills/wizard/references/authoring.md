# Authoring conventions

Library helpers: `stage`, `say`/`step`, `open_url`, `ask`/`ask_secret`, `write_env`, `set_secret`/`set_var`, `pause`/`confirm`.

Hold the bar the template sets:

- Open the URL **before** asking for its value.
- `ask_secret` for anything secret; `ask` for public values; some stages are pure actions and capture nothing.
- `write_env` every persisted value; `set_secret`/`set_var` only the values CI actually needs.
- `confirm` before any irreversible action.
- Each `stage` clears the screen so only the current step is visible: keep a stage to one focused task so nothing the human needs scrolls away.
- Stages run in dependency order — a stage never asks for a value an earlier stage hasn't produced.
- Don't touch the library above the `STAGES` marker. Only the stage definitions and `TOTAL_STAGES` are yours.
