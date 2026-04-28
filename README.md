# pow

<p align="center">
  <img src="assets/logo.png" alt="pow logo" width="240">
</p>

`pow` (project oriented workspace) is a Rust CLI that manages multi-repo workspaces via git worktrees. It groups worktrees from one or more source repos into a single directory, all checked out on a shared branch named after the workspace. Open one directory, run your tools (Claude Code, editor, formatter, build) scoped to exactly the repos you care about, all on the right branch.

## Install

```sh
cargo build --release
cp target/release/pow /usr/local/bin/pow   # or anywhere on $PATH
```

Requires `git` on `$PATH`.

## Quickstart

```sh
# 1. Register a source — a directory containing git clones.
pow source add babylist ~/src/Babylist

# 2. Create a workspace.
pow new studio

# 3. Enable shell integration (zsh) so `pow use` and `pow cd` can change dirs.
eval "$(pow init)"           # add this to ~/.zshrc for persistence

pow use studio               # sets $POW_ACTIVE, cds to ~/workspaces/studio/

# 4. Add repos. Each becomes a worktree checked out on branch `studio`.
pow add family-ties
pow add StudioOne

# 5. Navigate.
pow cd studio family-ties    # cd ~/workspaces/studio/family-ties

# 6. Work.
pow status                   # git status across all entries
pow exec "git log -1 --oneline"
pow sync                     # fetch in every entry's source clone

# 7. Tear down. Branches are left alone unless you pass --prune-branches.
pow rm studio --force
```

## Concepts

| Term       | What it is |
| ---------- | ---------- |
| Source     | A directory on disk (e.g. `~/src/Babylist/`) that contains real git clones. Optionally linked to a GitHub org. |
| Workspace  | A directory `~/workspaces/<name>/` holding worktrees cloned from one or more sources. |
| Entry      | A single worktree inside a workspace. Directory name matches the repo name. |

**Invariants:**

1. Workspace name = branch name (unless overridden per-entry with `-b`).
2. Workspaces live at `~/workspaces/<name>/`. Not configurable beyond `$POW_WORKSPACES_ROOT` (mainly for tests).
3. Sources are real clones; worktrees share their object database.
4. Removing a workspace removes worktrees. Branches stay unless you pass `--prune-branches`.

## Commands

### Workspace lifecycle

| Command | Description |
| ------- | ----------- |
| `pow new <name> [--force]` | Create empty workspace dir. |
| `pow add <repo> [-w <ws>] [-b <branch>] [-f <base>] [--no-setup]` | Add a repo as a worktree. |
| `pow forget <repo> [-w <ws>] [--prune-branch]` | Remove a worktree. |
| `pow rm <name> [--prune-branches] [--force]` | Tear down entire workspace. |
| `pow list [--json]` | List all workspaces. |
| `pow show [name] [--json] [--no-status]` | Show workspace contents + git status. |

### Navigation (require shell integration)

| Command | Description |
| ------- | ----------- |
| `pow use <name>` | Set active workspace for shell session. |
| `pow cd [name] [entry]` | cd into a workspace or entry. |
| `pow current [--json]` | Print the active workspace. |

### Working within a workspace

| Command | Description |
| ------- | ----------- |
| `pow switch <repo> <branch-or-commit> [--new]` | Switch an entry to a different ref. |
| `pow sync [repo] [--all] [-w <ws>]` | Fetch in the source clone(s). |
| `pow status [name] [--dirty-only] [--short]` | git status across entries. |
| `pow exec <cmd> [-w <ws>] [--parallel <n>] [--dry-run]` | Run a command in every entry dir. |

### Sources

| Command | Description |
| ------- | ----------- |
| `pow source add <name> <path> [--github-org <org>] [--base-branch main] [--include <glob>]... [--exclude <glob>]... [--all] [--skip-archived <bool>]` | Register (and optionally clone from GitHub). |
| `pow source list [--json]` | List registered sources. |
| `pow source sync <name> [--dry-run] [--prune] [--parallel <n>]` | Clone new org repos. |
| `pow source remove <name> [--force]` | Unregister. |

### Config / shell

| Command | Description |
| ------- | ----------- |
| `pow config [--json]` | Print full config. |
| `pow config get <key>` / `pow config set <key> <value>` | Supported keys: `settings.default_source`, `settings.parallel`, `github.token`. |
| `pow init` | Print zsh shell integration script. |
| `pow completions [zsh\|bash\|fish\|...]` | Print shell completion script. |

## Config file

`~/.config/pow/config.toml`:

```toml
[settings]
parallel = 4
# default_source = "babylist"

[github]
# token = "ghp_xxxx"    # or set $GITHUB_TOKEN

[[sources]]
name = "babylist"
path = "~/src/Babylist"
github_org = "babylist"
base_branch = "main"
skip_archived = true
include = ["web", "family-ties", "api-*"]
exclude = ["legacy-*"]
```

## Per-repo setup (`.pow.toml`)

Drop a `.pow.toml` at the root of any source repo to declare setup that runs whenever pow brings the repo into a workspace:

```toml
[setup]
commands = ["npm install", "bin/setup"]   # run sequentially in the new worktree on `pow add`
copy = [".env", ".env.local"]             # copied from source clone → worktree on `pow add` and `pow sync`
```

- `commands` run in the new worktree directory with live output. Failures print a warning but do not roll back the worktree.
- `copy` paths are relative; missing source files are skipped. Files are re-copied (overwriting) when `pow sync` runs against a workspace that contains the entry. `pow sync --all` skips re-copy.
- Pass `pow add --no-setup` to skip both steps.

## Shell integration

Add to `~/.zshrc`:

```sh
eval "$(pow init)"
```

For tab completions:

```sh
pow completions zsh > "${fpath[1]}/_pow"
# or: pow completions zsh > ~/.zfunc/_pow  (and add ~/.zfunc to $fpath)
```

The zsh completion is dynamic — it tab-completes workspace names, entries
within a workspace, repos from your registered sources, source names, and
config keys. `pow completions bash|fish|powershell` emit static
clap-generated scripts (subcommand and flag names only).

## Environment variables

| Variable | Purpose |
| -------- | ------- |
| `POW_ACTIVE` | Active workspace, set by `pow use`. |
| `POW_CONFIG` | Override config file path. |
| `POW_WORKSPACES_ROOT` | Override `~/workspaces/` (mainly for tests). |
| `GITHUB_TOKEN` | Used when `github.token` is unset. |
| `POW_LOG` | `tracing` filter (e.g. `debug`). |

## Exit codes

| Code | Meaning |
| ---- | ------- |
| 0 | success |
| 1 | general error |
| 2 | workspace not found |
| 3 | repo/entry not found or ambiguous |
| 4 | source not found |
| 5 | git / worktree operation failed |
| 6 | GitHub API error |

## License

MIT.
