# Changelog

## Unreleased

- Workspace templates. Define `[[templates]]` blocks in the global config (`name` + `repos`) and bootstrap a workspace in one command: `pow new <ws> -t <template>`. Accepts `-f/--from` and `--no-setup`, applied to every repo in the template. Failures continue and are reported in a summary; the command exits non-zero if any repo failed. New `pow template list` and tab completion for `-t`.
- Per-repo setup hooks via a committed `.pow.toml`. `pow add` runs `[setup].commands` and copies `[setup].copy` files from the source clone into the new worktree; `pow sync` re-copies those files. Skip with `pow add --no-setup`.

## 0.1.0

Initial release.

- Workspace lifecycle: `pow new`, `pow add`, `pow forget`, `pow rm`, `pow list`, `pow show`.
- Navigation with zsh shell integration: `pow use`, `pow cd`, `pow current`.
- Working within a workspace: `pow switch`, `pow sync`, `pow status`, `pow exec`.
- Source management: `pow source add`, `pow source list`, `pow source sync`, `pow source remove`.
- GitHub org integration with interactive picker, include/exclude glob filters, and parallel cloning.
- Configuration in `~/.config/pow/config.toml` with `pow config` commands.
- `pow init` shell script and `pow completions` for tab completion.
