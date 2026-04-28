#compdef pow
# pow zsh completion — install with:  pow completions zsh > "${fpath[1]}/_pow"
#
# This script is hand-tuned to call back into `pow __complete <kind>` for
# dynamic value completion (workspace names, entries, repos, sources, config
# keys). Static subcommand and flag names are listed inline so completion stays
# fast even when no shell integration is loaded.

_pow_workspaces() {
    local -a items
    items=(${(f)"$(command pow __complete workspaces 2>/dev/null)"})
    _describe -t workspaces 'workspace' items
}

_pow_entries() {
    local ws=${opt_args[-w]:-${opt_args[--workspace]}}
    local -a items args
    args=(entries)
    [[ -n $ws ]] && args+=(--workspace $ws)
    items=(${(f)"$(command pow __complete ${args[@]} 2>/dev/null)"})
    _describe -t entries 'entry' items
}

_pow_repos() {
    local -a items
    items=(${(f)"$(command pow __complete repos 2>/dev/null)"})
    _describe -t repos 'repo' items
}

_pow_sources() {
    local -a items
    items=(${(f)"$(command pow __complete sources 2>/dev/null)"})
    _describe -t sources 'source' items
}

_pow_templates() {
    local -a items
    items=(${(f)"$(command pow __complete templates 2>/dev/null)"})
    _describe -t templates 'template' items
}

_pow_config_keys() {
    local -a items
    items=(${(f)"$(command pow __complete config-keys 2>/dev/null)"})
    _describe -t config-keys 'config key' items
}

_pow_shells() {
    local -a items
    items=(bash elvish fish powershell zsh)
    _describe -t shells 'shell' items
}

_pow_commands() {
    local -a commands
    commands=(
        'new:Create a new (empty) workspace'
        'add:Add a repo as a worktree in a workspace'
        'forget:Remove a worktree from a workspace'
        'rm:Tear down an entire workspace'
        'list:List all workspaces'
        'show:Show the contents of a workspace'
        'use:Set the active workspace'
        'cd:cd into a workspace or entry'
        'current:Print the active workspace'
        'switch:Switch an entry to a different branch or commit'
        'sync:Fetch in the underlying source clones'
        'status:Git status across entries in a workspace'
        'exec:Run a command in every entry directory'
        'source:Manage sources'
        'template:Manage workspace templates'
        'config:Print, get, or set configuration'
        'init:Print zsh shell integration script'
        'completions:Print shell completion script'
        'help:Show help'
    )
    _describe -t commands 'pow command' commands
}

_pow_source_commands() {
    local -a commands
    commands=(
        'add:Register a source directory'
        'list:List registered sources'
        'sync:Clone new repos from a source'
        'remove:Unregister a source'
    )
    _describe -t source-commands 'source subcommand' commands
}

_pow_template_commands() {
    local -a commands
    commands=(
        'list:List configured templates'
    )
    _describe -t template-commands 'template subcommand' commands
}

_pow_config_commands() {
    local -a commands
    commands=(
        'get:Get a single config value'
        'set:Set a single config value'
    )
    _describe -t config-commands 'config subcommand' commands
}

_pow_source() {
    local context state state_descr line
    typeset -A opt_args
    _arguments -C \
        '1: :_pow_source_commands' \
        '*::source-arg:->source-arg'

    case $state in
        source-arg)
            case $words[1] in
                add)
                    _arguments \
                        '--github-org=[GitHub org to clone from]:org' \
                        '--base-branch=[Base branch]:branch:(main master)' \
                        '*--include=[Include glob]:pattern' \
                        '*--exclude=[Exclude glob]:pattern' \
                        '--all[Skip interactive picker]' \
                        '--skip-archived=[Skip archived repos]:bool:(true false)' \
                        '1:source name' \
                        '2:path:_files -/'
                    ;;
                list)
                    _arguments '--json[Emit JSON]'
                    ;;
                sync)
                    _arguments \
                        '--dry-run[Show what would happen]' \
                        '--prune[Remove local repos no longer matched]' \
                        '--parallel=[Parallel clones]:n' \
                        '1: :_pow_sources'
                    ;;
                remove)
                    _arguments \
                        '--force[Skip confirmation]' \
                        '1: :_pow_sources'
                    ;;
            esac
            ;;
    esac
}

_pow_template() {
    local context state state_descr line
    typeset -A opt_args
    _arguments -C \
        '1: :_pow_template_commands' \
        '*::template-arg:->template-arg'

    case $state in
        template-arg)
            case $words[1] in
                list)
                    _arguments '--json[Emit JSON]'
                    ;;
            esac
            ;;
    esac
}

_pow_config() {
    local context state state_descr line
    typeset -A opt_args
    _arguments -C \
        '--json[Emit JSON]' \
        '1: :_pow_config_commands' \
        '*::config-arg:->config-arg'

    case $state in
        config-arg)
            case $words[1] in
                get)
                    _arguments '1: :_pow_config_keys'
                    ;;
                set)
                    _arguments \
                        '1: :_pow_config_keys' \
                        '2:value'
                    ;;
            esac
            ;;
    esac
}

_pow() {
    local context state state_descr line
    typeset -A opt_args
    _arguments -C \
        '(- *)'{-h,--help}'[Show help]' \
        '(- *)'{-V,--version}'[Show version]' \
        '1: :_pow_commands' \
        '*::arg:->arg'

    case $state in
        arg)
            case $words[1] in
                new)
                    _arguments \
                        '--force[Recreate if exists]' \
                        '(-t --template)'{-t+,--template=}'[Template to apply]: :_pow_templates' \
                        '(-f --from)'{-f+,--from=}'[Base branch/ref]:base ref' \
                        '--no-setup[Skip per-repo setup hooks]' \
                        '1:workspace name'
                    ;;
                add)
                    _arguments \
                        '(-w --workspace)'{-w+,--workspace=}'[Workspace]: :_pow_workspaces' \
                        '(-b --branch)'{-b+,--branch=}'[Branch]:branch' \
                        '(-f --from)'{-f+,--from=}'[Base branch/ref]:base ref' \
                        '--no-setup[Skip per-repo setup hooks]' \
                        '1: :_pow_repos'
                    ;;
                forget)
                    _arguments \
                        '(-w --workspace)'{-w+,--workspace=}'[Workspace]: :_pow_workspaces' \
                        '--prune-branch[Also delete the branch if safe]' \
                        '1: :_pow_entries'
                    ;;
                rm)
                    _arguments \
                        '--prune-branches[Also delete each entry'\''s branch]' \
                        '--force[Skip confirmation]' \
                        '1: :_pow_workspaces'
                    ;;
                list)
                    _arguments '--json[Emit JSON]'
                    ;;
                show)
                    _arguments \
                        '--json[Emit JSON]' \
                        '--no-status[Skip git status]' \
                        '1: :_pow_workspaces'
                    ;;
                use)
                    _arguments '1: :_pow_workspaces'
                    ;;
                cd)
                    _arguments \
                        '1: :_pow_workspaces' \
                        '2: :_pow_entries'
                    ;;
                current)
                    _arguments '--json[Emit JSON]'
                    ;;
                switch)
                    _arguments \
                        '(-w --workspace)'{-w+,--workspace=}'[Workspace]: :_pow_workspaces' \
                        '--new[Create a new branch from current HEAD]' \
                        '1: :_pow_entries' \
                        '2:branch or commit'
                    ;;
                sync)
                    _arguments \
                        '(-w --workspace)'{-w+,--workspace=}'[Workspace]: :_pow_workspaces' \
                        '--all[Fetch every source in config]' \
                        '1: :_pow_entries'
                    ;;
                status)
                    _arguments \
                        '--dirty-only[Only show dirty entries]' \
                        '--short[Compact output]' \
                        '1: :_pow_workspaces'
                    ;;
                exec)
                    _arguments \
                        '(-w --workspace)'{-w+,--workspace=}'[Workspace]: :_pow_workspaces' \
                        '--parallel=[How many to run in parallel]:n' \
                        '--dry-run[Print without executing]' \
                        '1:command'
                    ;;
                source)
                    _pow_source
                    ;;
                template)
                    _pow_template
                    ;;
                config)
                    _pow_config
                    ;;
                completions)
                    _arguments '1: :_pow_shells'
                    ;;
                init|current|list)
                    ;;
            esac
            ;;
    esac
}

_pow "$@"
