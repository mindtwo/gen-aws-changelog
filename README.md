# aws-utils

AWS pipeline management utility for mindtwo. Compares deployed stages,
generates JIRA-enriched changelogs, approves manual-approval releases,
runs multi-pipeline recipes, and audits S3 buckets. Ships as a single
static binary; also includes a read-only ratatui browser (`aws-utils
tui`) over projects, recipes, and accounts.

## Install

### From a GitHub release (recommended)

Each tagged release attaches prebuilt binaries for Linux (x86_64,
aarch64) and macOS (x86_64, aarch64) at:

```
https://github.com/mindtwo/gen-aws-changelog/releases
```

Download the tarball for your platform, extract it, and put `aws-utils`
on your `$PATH`. Each archive ships with a `.sha256` checksum.

### From source

Requires Rust 1.80+.

```bash
cargo install --git https://github.com/mindtwo/gen-aws-changelog --branch v2
```

### Local checkout + symlink

For active development or if you want to track `v2` directly from a
local working copy.

1. **Check out and build** the release binary:

   ```bash
   git clone https://github.com/mindtwo/gen-aws-changelog
   cd gen-aws-changelog
   git checkout v2
   cargo build --release
   # binary lands at target/release/aws-utils
   ```

2. **Link the binary** into a directory on your `$PATH` so updates after
   each `cargo build --release` are picked up automatically:

   ```bash
   ln -sf "$(pwd)/target/release/aws-utils" /usr/local/bin/aws-utils
   # verify
   aws-utils --version
   ```

   On systems where `/usr/local/bin` is not writable without sudo, link
   into `~/.local/bin` (or any other `$PATH` entry) instead:

   ```bash
   ln -sf "$(pwd)/target/release/aws-utils" ~/.local/bin/aws-utils
   ```

## Credentials

`aws-utils` reads credentials from the environment (a local `.env` file is
auto-loaded). Copy `.env.example` and fill in the values you need:

| Variable | Purpose |
|---|---|
| `AWS_*` / `AWS_PROFILE` | Standard AWS credential chain |
| `GITHUB_TOKEN` | GitHub REST API; falls back to `gh auth token` |
| `JIRA_BASE_URL`, `JIRA_EMAIL`, `JIRA_API_TOKEN` | Required for `changelog` ticket enrichment (or run `aws-utils jira configure`) |

## Commands

```bash
aws-utils add                       # register the current dir as a project
aws-utils config show               # print the effective config
aws-utils config edit               # open .aws-utils.toml in $EDITOR
aws-utils config push               # commit + push .aws-utils.toml
aws-utils config pull               # pull .aws-utils.toml from the repo
aws-utils accounts add NAME [--description "..."]
aws-utils accounts list             # see your configured AWS account names
aws-utils accounts remove NAME
aws-utils jira configure            # interactively store JIRA URL + token
aws-utils jira test                 # verify the credentials against /myself
aws-utils jira show                 # print resolved settings (token masked)
aws-utils assume [ACCOUNT]          # eval-style: eval "$(aws-utils assume X)"
aws-utils check                     # compare deployed commits between stages
aws-utils changelog --out FILE      # render markdown changelog (commits + JIRA)
aws-utils release                   # changelog → approve → annotated tag
aws-utils recipe create <name>      # bundle multiple projects into a recipe
aws-utils recipe list
aws-utils recipe run <name>         # sequential release with confirmation
aws-utils s3-check paths.txt --bucket B [--project NAME]
```

## AWS authentication via `assume-role`

`aws-utils` integrates with the `assume-role` shell script. Once accounts
are registered globally and named in a project's config, commands that
talk to AWS auto-run `assume-role` (prompting once for MFA) before any
SDK call. Auto-assume is skipped if `AWS_SESSION_TOKEN` is already set
in your environment, so manually-assumed sessions are respected.

```bash
# 1. Register accounts once
aws-utils accounts add prod-app-teach --description "production"
aws-utils accounts add media-app-teach --description "media bucket account"

# 2. Map them to action groups in .aws-utils.toml (per-project)
[aws]
release = "prod-app-teach"   # used by check / changelog / release
s3      = "media-app-teach"  # used by s3-check
# default = "prod-app-teach" # optional fallback for any action

# 3. Run commands normally — MFA is prompted once per process
aws-utils check
aws-utils s3-check paths.txt --bucket media-bucket
```

The assume-role binary path defaults to `/usr/local/bin/assume-role`;
override with `AWS_UTILS_ASSUME_ROLE` if it lives elsewhere.

### Exporting credentials into your shell

Because `aws-utils` is a separate process, `aws-utils assume <account>`
on its own can't modify your shell's environment — a child process
never can. Two ways to make it stick:

**Recommended: install the shell wrapper** (defines an `awsu` function
that does the `eval` for you):

```bash
# ~/.zshrc  (or ~/.bashrc)
eval "$(aws-utils init zsh)"   # or `bash`

# fish
aws-utils init fish | source
```

Then:

```bash
awsu assume prod-app-teach     # exports AWS_* into the current shell
awsu tui                        # if you assume in the TUI, it loads on exit
```

**Manual: eval directly**

```bash
eval "$(aws-utils assume prod-app-teach)"
```

Every successful assume (CLI, auto-assume, or TUI) also writes the
session to `~/.cache/aws-utils/session.sh` (mode `0600`). The shell
wrapper sources and immediately deletes this file after `awsu assume`
or `awsu tui`, so credentials don't sit on disk longer than the
hand-off needs. If you `aws-utils assume` directly (without the
wrapper), you can re-load the file with:

```bash
eval "$(aws-utils session)"     # or: . ~/.cache/aws-utils/session.sh
```

### Logging out

`aws-utils logout` clears the assumed session: it prints `unset`
statements for every credential variable and removes the on-disk
session file.

```bash
awsu logout                     # via wrapper — env cleared in current shell
eval "$(aws-utils logout)"      # manual equivalent
```

All commands accept `--region`, `-v/--verbose`, `-q/--quiet`, and
`--no-color`.

## Project configuration

After `aws-utils add`, an `.aws-utils.toml` is written into the project
root. Commit it. It looks like this:

```toml
pipeline = "app-learning-frontend"
region = "eu-central-1"
from_stage = "DeployPreProd"
to_stage = "DeployProd"

[jira]
prefixes = ["LEARN", "APP"]                       # JIRA project keys to query
statuses = ["Ready for Release", "Done"]          # workflow stages to include
```

The global registry (`~/.config/aws-utils/projects/<name>.toml` on
Linux, `~/Library/Application Support/aws-utils/projects/` on macOS) is
what lets `aws-utils` find your project from any directory. Recipes
live next to it under `recipes/`.

## Development

The repo uses GitHub Actions for CI (`.github/workflows/ci.yml`) and
release builds (`.github/workflows/release.yml`). CI runs the same
three checks locally:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Cutting a release: push a tag matching `v*` (e.g. `v0.2.0`). The
release workflow builds for `x86_64-unknown-linux-gnu`,
`aarch64-unknown-linux-gnu`, `x86_64-apple-darwin`,
`aarch64-apple-darwin` and publishes a GitHub release with the
tarballs + sha256 checksums attached. Pre-release tags
(e.g. `v0.2.0-rc1`) are auto-marked as prereleases.

## License

MIT
