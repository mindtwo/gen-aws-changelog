import { defineCommand, runMain } from 'citty';
import { compareCommits, parseReponame } from '../src/get-repo.mjs';
import { getAwsPipelineStages } from '../src/pipeline-commits.mjs';
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
                'The repository name with organization (e.g., org/repo)',
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

        // // Get the changelog via aws pipeline
        // const { fromStage, toStage } =
        //     (await getAwsPipelineStages(config)) || {};

        // if (!fromStage || !toStage) {
        //     consola.error('No stage information found in the pipeline.');
        //     process.exit(1);
        // }

        // // TODO: return a list of commits
        // const changelog = compareCommits(
        //     repo,
        //     toStage.revisionId,
        //     fromStage.revisionId
        // );

        // if (!changelog || changelog.length === 0) {
        //     consola.warn('No changes found between the specified stages.');
        //     return;
        // }

        // const fromSha = fromStage.revisionId.slice(0, 7);
        // const toSha = toStage.revisionId.slice(0, 7);

        // const header = `### Changes for release ${fromStage.stageName} (${fromSha}) to ${toStage.stageName} (${toSha}):`;

        // // Print the changelog
        // console.log(`${header}\n\n${changelog}\n`);
    },
});

runMain(main);
