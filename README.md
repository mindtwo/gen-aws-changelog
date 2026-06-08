# aws-utils

AWS pipeline management utility for mindtwo. Compares deployed stages,
generates JIRA-enriched changelogs, approves manual-approval releases,
runs multi-pipeline recipes, and audits S3 buckets.

> **v2 status (this branch):** Rust rewrite of the original Node.js
> `@mindtwo/gen-aws-changelog`. Core CLI is functional. The interactive
> TUI (`aws-utils tui`) and CI release workflow are not yet implemented —
> see the plan file referenced in commit history.

## Install

Requires Rust 1.80+.

```bash
cargo install --git https://github.com/mindtwo/gen-aws-changelog --branch v2
```

Or build from source:

```bash
git clone https://github.com/mindtwo/gen-aws-changelog
cd gen-aws-changelog
git checkout v2
cargo build --release
# binary at target/release/aws-utils
```

## Credentials

`aws-utils` reads credentials from the environment (a local `.env` file is
auto-loaded). Copy `.env.example` and fill in the values you need:

| Variable | Purpose |
|---|---|
| `AWS_*` / `AWS_PROFILE` | Standard AWS credential chain |
| `GITHUB_TOKEN` | GitHub REST API; falls back to `gh auth token` |
| `JIRA_BASE_URL`, `JIRA_EMAIL`, `JIRA_API_TOKEN` | Required for `changelog` ticket enrichment |

## Commands

```bash
aws-utils add                       # register the current dir as a project
aws-utils config show               # print the effective config
aws-utils config edit               # open .aws-utils.toml in $EDITOR
aws-utils config push               # commit + push .aws-utils.toml
aws-utils config pull               # pull .aws-utils.toml from the repo
aws-utils check                     # compare deployed commits between stages
aws-utils changelog --out FILE      # render markdown changelog (commits + JIRA)
aws-utils release                   # changelog → approve → annotated tag
aws-utils recipe create <name>      # bundle multiple projects into a recipe
aws-utils recipe list
aws-utils recipe run <name>         # sequential release with confirmation
aws-utils s3-check paths.txt --bucket B
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
prefixes = ["LEARN", "APP"]   # only extract these keys from commits
```

The global registry (`~/.config/aws-utils/projects/<name>.toml` on
Linux, `~/Library/Application Support/aws-utils/projects/` on macOS) is
what lets `aws-utils` find your project from any directory. Recipes
live next to it under `recipes/`.

## License

MIT
