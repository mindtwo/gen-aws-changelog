import { defineCommand, runMain } from 'citty';
import { compareCommits, createRelease } from '../src/github-cli.mjs';
import { getAwsPipelineStages } from '../src/pipeline-commits.mjs';
import { parseReponame } from '../src/util.mjs';
import consola from 'consola';
import { parse } from '../src/config.mjs';
import { checkPrerequisites } from '../src/dependencies.mjs';

const main = defineCommand({
    meta: {
        name: 'gen-aws-changelog',
        version: '1.0.0',
        description: 'Generate AWS changelog from pipeline commits',
    },
    args: {
        repo: {
            type: 'positional',
            description:
                'The repository name with organization/owner (e.g., owner/repo)',
            required: true,
        },
        pipeline: {
            type: 'string',
            description: 'The name of the AWS CodePipeline (optional)',
        },
        fromStage: {
            type: 'string',
            description: 'The starting stage of the pipeline (optional)',
        },
        toStage: {
            type: 'string',
            description: 'The ending stage of the pipeline (optional)',
        },
        region: {
            type: 'string',
            description:
                'AWS region for the CodePipeline (default: eu-central-1)',
            default: 'eu-central-1',
        },
        verbose: {
            type: 'boolean',
            description: 'Enable verbose logging',
            default: false,
        },
        quiet: {
            type: 'boolean',
            description: 'Only show errors',
            default: false,
        },
        noGit: {
            type: 'boolean',
            description: "Don't use git to fetch the repository configuration",
            default: false,
        },
        tag: {
            type: 'boolean',
            description:
                'Create a commit for the current date on master with the changelog as description',
            default: false,
        },
    },
    async run({ args }) {
        // configure logger level based on flags
        if (args.verbose) {
            consola.level = 4; // Debug
        } else {
            consola.level = 3; // Error
        }

        if (!checkPrerequisites()) {
            process.exit(1);
        }

        const repo = parseReponame(args.repo);
        const config = parse(args);

        if (!config) {
            consola.error('Configuration is missing or invalid.');
            process.exit(1);
        }

        // Get the changelog via aws pipeline
        const { fromStage, toStage } =
            (await getAwsPipelineStages(config)) || {};

        if (!fromStage || !toStage) {
            consola.error('No stage information found in the pipeline.');
            process.exit(1);
        }

        // Get formatted commit messages
        const changelog = compareCommits(
            repo,
            toStage.revisionId,
            fromStage.revisionId
        );

        if (!changelog || changelog.length === 0) {
            consola.warn('No changes found between the specified stages.');
            return;
        }

        const fromSha = fromStage.revisionId.slice(0, 7);
        const toSha = toStage.revisionId.slice(0, 7);

        const title = `Changes for release ${fromStage.stageName} (${fromSha}) to ${toStage.stageName} (${toSha})`;

        if (args.tag) {
            const date = new Date();
            const formattedDate = `${date.getDate()}-${
                date.getMonth() + 1
            }-${date.getFullYear()}`;

            const tag = `release-${formattedDate}`;

            const url = createRelease(
                `release-${formattedDate}`,
                title,
                changelog.join('\n'),
                args.repo
            )?.trim();

            consola.info(`Created release ${tag} (${url})`);
        }

        const header = `### ${title}:`;

        // Print the changelog
        console.log(`\n${header}\n\n${changelog.join('\n')}\n`);
    },
});

runMain(main);
