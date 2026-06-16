//! `aws-utils init <shell>` — prints a shell function that wraps
//! `aws-utils` so `assume` and `tui` actually update the calling
//! shell's environment.
//!
//! Bash/zsh share the same `eval`-based pattern. Fish needs a separate
//! shape because it doesn't have `eval $(...)` and uses `set -gx` for
//! exports — for fish we suggest a one-shot recipe instead of trying to
//! re-implement the wrapper.

use crate::cli::{InitArgs, Shell};
use crate::error::Result;

pub async fn run(args: InitArgs) -> Result<()> {
    let snippet = match args.shell {
        Shell::Bash | Shell::Zsh => POSIX_WRAPPER,
        Shell::Fish => FISH_WRAPPER,
    };
    print!("{snippet}");
    Ok(())
}

const POSIX_WRAPPER: &str = r#"# aws-utils shell wrapper. Source from your shell rc:
#   eval "$(aws-utils init zsh)"   # or bash
#
# Subcommands handled specially:
#   awsu assume <account>   eval exports into current shell, then drop the session file
#   awsu logout             eval unset statements, drop the session file
#   awsu tui                run TUI; if a session was assumed inside it, source it then delete
# Everything else passes through to `aws-utils` directly.

awsu() {
    _awsu_session() {
        local p="${XDG_CACHE_HOME:-$HOME/.cache}/aws-utils/session.sh"
        [ -r "$p" ] || p="$HOME/Library/Caches/aws-utils/session.sh"
        [ -r "$p" ] && printf '%s' "$p"
    }
    case "${1:-}" in
        assume)
            local _out _rc
            _out=$(command aws-utils "$@")
            _rc=$?
            if [ $_rc -eq 0 ]; then
                eval "$_out"
                # Rust wrote the session file as a backstop for the TUI
                # path. We've already eval'd it, so drop it to keep
                # creds off disk.
                local _s
                _s=$(_awsu_session)
                [ -n "$_s" ] && rm -f "$_s"
            else
                printf '%s\n' "$_out" >&2
            fi
            return $_rc
            ;;
        logout)
            local _out _rc
            _out=$(command aws-utils logout)
            _rc=$?
            [ $_rc -eq 0 ] && eval "$_out"
            return $_rc
            ;;
        tui)
            command aws-utils "$@"
            local _rc=$? _s
            _s=$(_awsu_session)
            if [ -n "$_s" ]; then
                # shellcheck disable=SC1090
                . "$_s"
                rm -f "$_s"
            fi
            return $_rc
            ;;
        *)
            command aws-utils "$@"
            ;;
    esac
}
"#;

const FISH_WRAPPER: &str = r#"# aws-utils shell wrapper for fish. Source from ~/.config/fish/config.fish:
#   aws-utils init fish | source
#
# Same lifecycle as the bash/zsh wrapper: assume/tui/logout consume
# the session file then delete it; other subcommands pass through.

function _awsu_apply_line --no-scope-shadowing
    set -l line $argv[1]
    if string match -q 'export *' -- $line
        set -l kv (string replace -r '^export ([A-Z_][A-Z0-9_]*)="(.*)";?$' '$1 $2' -- $line)
        if test -n "$kv"
            set -gx (string split ' ' -- $kv)
        end
    else if string match -q 'unset *' -- $line
        set -l key (string replace -r '^unset ([A-Z_][A-Z0-9_]*);?$' '$1' -- $line)
        if test -n "$key"
            set -e $key
        end
    end
end

function _awsu_session_path
    if test -r "$HOME/.cache/aws-utils/session.sh"
        echo "$HOME/.cache/aws-utils/session.sh"
    else if test -r "$HOME/Library/Caches/aws-utils/session.sh"
        echo "$HOME/Library/Caches/aws-utils/session.sh"
    end
end

function awsu
    switch $argv[1]
        case assume
            set -l out (command aws-utils $argv)
            set -l rc $status
            if test $rc -eq 0
                for line in $out
                    _awsu_apply_line $line
                end
                set -l s (_awsu_session_path)
                test -n "$s"; and rm -f "$s"
            else
                printf '%s\n' $out 1>&2
            end
            return $rc
        case logout
            set -l out (command aws-utils logout)
            set -l rc $status
            if test $rc -eq 0
                for line in $out
                    _awsu_apply_line $line
                end
            end
            return $rc
        case tui
            command aws-utils $argv
            set -l rc $status
            set -l s (_awsu_session_path)
            if test -n "$s"
                for line in (cat $s)
                    _awsu_apply_line $line
                end
                rm -f "$s"
            end
            return $rc
        case '*'
            command aws-utils $argv
    end
end
"#;
