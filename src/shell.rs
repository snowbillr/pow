use crate::error::Result;

const ZSH_INIT: &str = r#"# pow shell integration (zsh)
# Add to your ~/.zshrc:   eval "$(pow init)"
pow() {
    case "$1" in
        use)
            if [ -z "$2" ]; then
                command pow use 2>&1
                return 1
            fi
            local __pow_name
            __pow_name="$(command pow __resolve-use "$2")" || return 1
            export POW_ACTIVE="$__pow_name"
            cd "${POW_WORKSPACES_ROOT:-$HOME/workspaces}/$__pow_name" || return 1
            ;;
        cd)
            local __pow_target
            __pow_target="$(command pow __resolve-cd "${@:2}")" || return 1
            cd "$__pow_target" || return 1
            ;;
        *)
            command pow "$@"
            ;;
    esac
}
"#;

pub fn print_shell_init() -> Result<()> {
    let shell = std::env::var("SHELL").unwrap_or_default();
    if !shell.ends_with("zsh") && !shell.is_empty() {
        eprintln!(
            "warning: pow shell integration is tuned for zsh; your $SHELL is '{shell}'. \
             The script below is emitted anyway."
        );
    }
    print!("{ZSH_INIT}");
    Ok(())
}
