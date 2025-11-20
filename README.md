# gen-aws-changelog

Generate changelogs by comparing commits between AWS CodePipeline stages for your GitHub repositories.

## Overview

`gen-aws-changelog` is a CLI tool that fetches the current deployment state from AWS CodePipeline stages and generates a changelog showing all commits between two stages (e.g., staging and production). It uses the GitHub CLI to fetch commit information and compare revisions.

## Prerequisites

Before using this tool, ensure you have:

1. **GitHub CLI (`gh`)** installed and authenticated
   ```bash
   # Install gh CLI (macOS)
   brew install gh

   # Authenticate
   gh auth login
   ```

2. **AWS credentials** configured with permissions to read CodePipeline state
   ```bash
   # Configure AWS credentials
   aws configure
   ```

   Required AWS permissions:
   - `codepipeline:GetPipelineState`
   - `codepipeline:GetPipelineExecution`

3. **Node.js** (version 20 or higher)

## Installation

```bash
npm install -g @mindtwo/gen-aws-changelog
```

## Usage

### Basic Command

```bash
gen-aws-changelog <org/repo> --pipeline <pipeline-name> --fromStage <stage1> --toStage <stage2>
```

### Example

```bash
gen-aws-changelog myorg/myrepo \
  --pipeline my-production-pipeline \
  --fromStage DeployStaging \
  --toStage DeployProduction \
  --region us-east-1
```

This will output:

```
### Changes for release DeployStaging (abc1234) to DeployProduction (def5678):

- abc1234 Fix authentication bug
- bcd2345 Add user profile feature
- cde3456 Update dependencies
```

## Command-Line Options

| Option | Type | Required | Default | Description |
|--------|------|----------|---------|-------------|
| `repo` | positional | Yes | - | Repository name with organization/owner (e.g., `owner/repo`) |
| `--pipeline` | string | No* | - | AWS CodePipeline name |
| `--fromStage` | string | No* | - | Starting stage of the pipeline (e.g., `DeployStaging`) |
| `--toStage` | string | No* | - | Ending stage of the pipeline (e.g., `DeployProduction`) |
| `--region` | string | No | `eu-central-1` | AWS region for CodePipeline |
| `--verbose` | boolean | No | `false` | Enable verbose logging |
| `--quiet` | boolean | No | `false` | Only show errors |
| `--noGit` | boolean | No | `false` | Don't fetch repository configuration from GitHub |

\* Required unless configured in repository (see Git Repository Configuration below)

## Git Repository Configuration

To avoid specifying pipeline configuration on every command, you can configure your repository with a `.aws-changelog.json` file. This file is automatically fetched from the default branch of your repository.

### Configuration File

Create a file named `.aws-changelog.json` in the root of your repository:

```json
{
  "pipeline": "my-production-pipeline",
  "region": "us-east-1",
  "fromStage": "DeployStaging",
  "toStage": "DeployProduction"
}
```

### Configuration Schema

```typescript
{
  "pipeline": string,     // CodePipeline name (optional)
  "region": string,       // AWS region (optional, defaults to eu-central-1)
  "fromStage": string,    // Starting stage name (optional)
  "toStage": string       // Ending stage name (optional)
}
```

### Configuration Priority

The tool merges configuration from multiple sources in this order (later sources override earlier ones):

1. Default values
2. `.aws-changelog.json` from repository (if `--noGit` is not set)
3. Command-line arguments

### Example with Repository Configuration

Once you've added `.aws-changelog.json` to your repository, you can run:

```bash
# Uses configuration from .aws-changelog.json
gen-aws-changelog myorg/myrepo

# Override specific options
gen-aws-changelog myorg/myrepo --toStage DeployCanary
```

### Bypass Repository Configuration

Use `--noGit` to ignore the repository configuration file:

```bash
gen-aws-changelog myorg/myrepo --noGit \
  --pipeline different-pipeline \
  --fromStage Stage1 \
  --toStage Stage2
```

## How It Works

1. **Fetch Repository Config** (optional): Uses `gh` CLI to fetch `.aws-changelog.json` from the repository's default branch
2. **Query AWS CodePipeline**: Fetches the current state of the specified pipeline stages
3. **Extract Commit SHAs**: Gets the Git commit SHAs deployed to each stage
4. **Compare Commits**: Uses `gh` CLI to compare commits between the two SHAs
5. **Generate Changelog**: Formats and displays the commit messages

## Scripts

```bash
# Generate changelog
gen-aws-changelog myorg/myrepo --pipeline my-pipeline --fromStage Stage1 --toStage Stage2
```

## Error Handling

The tool will exit with an error if:

- Repository name format is invalid (must be `org/repo`)
- Required configuration is missing
- AWS CodePipeline stages are not found
- GitHub repository is not accessible
- AWS credentials are not configured or lack permissions

## Troubleshooting

### "Stage not found in pipeline"

Ensure the stage names match exactly (case-sensitive) with your CodePipeline configuration.

```bash
# List pipeline stages
aws codepipeline get-pipeline-state --name your-pipeline-name
```

### "Error fetching file from repository"

Ensure:
- `gh` CLI is authenticated (`gh auth status`)
- You have access to the repository
- The repository exists and the name is correct

### AWS Permission Issues

Verify your AWS credentials have the required permissions:

```bash
aws codepipeline get-pipeline-state --name your-pipeline-name
```

## License

MIT
