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
# `awsu assume <account>` runs the wrapper-aware assume and exports
# credentials into the current shell. `awsu tui` does the same after
# the TUI exits if you assumed inside it.

awsu() {
    case "${1:-}" in
        assume)
            local _out _rc
            _out=$(command aws-utils "$@")
            _rc=$?
            if [ $_rc -eq 0 ]; then
                eval "$_out"
            else
                printf '%s\n' "$_out" >&2
            fi
            return $_rc
            ;;
        tui)
            command aws-utils "$@"
            local _rc=$?
            local _session="${XDG_CACHE_HOME:-$HOME/.cache}/aws-utils/session.sh"
            # macOS users have ~/Library/Caches/aws-utils/session.sh instead
            if [ ! -r "$_session" ] && [ -r "$HOME/Library/Caches/aws-utils/session.sh" ]; then
                _session="$HOME/Library/Caches/aws-utils/session.sh"
            fi
            if [ -r "$_session" ]; then
                # shellcheck disable=SC1090
                . "$_session"
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
# `awsu assume <account>` runs the wrapper-aware assume and exports
# credentials into the current shell.

function awsu
    switch $argv[1]
        case assume
            set -l out (command aws-utils $argv)
            set -l rc $status
            if test $rc -eq 0
                for line in $out
                    if string match -q 'export *' -- $line
                        set -l kv (string replace -r '^export ([A-Z_][A-Z0-9_]*)="(.*)";?$' '$1 $2' -- $line)
                        if test -n "$kv"
                            set -gx (string split ' ' -- $kv)
                        end
                    end
                end
            else
                printf '%s\n' $out 1>&2
            end
            return $rc
        case tui
            command aws-utils $argv
            set -l rc $status
            set -l session "$HOME/.cache/aws-utils/session.sh"
            if not test -r "$session"
                set session "$HOME/Library/Caches/aws-utils/session.sh"
            end
            if test -r "$session"
                # Parse session file the same way as `assume` above.
                for line in (cat $session)
                    if string match -q 'export *' -- $line
                        set -l kv (string replace -r '^export ([A-Z_][A-Z0-9_]*)="(.*)";?$' '$1 $2' -- $line)
                        if test -n "$kv"
                            set -gx (string split ' ' -- $kv)
                        end
                    end
                end
            end
            return $rc
        case '*'
            command aws-utils $argv
    end
end
"#;
