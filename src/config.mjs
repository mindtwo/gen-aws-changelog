import { pipe, z } from 'zod';
import { consola } from 'consola';
import { getGithubFile } from './get-repo.mjs';

// Schema for validating GitHub configuration
const GithubConfigSchema = z.object({
    pipeline: z.string().optional(),
    region: z.string().optional(),
    fromStage: z.string().optional(),
    toStage: z.string().optional(),
});

// Function to validate the configuration against the schema
const ConfigSchema = z.object({
    pipeline: z.string().min(1, 'Repository name is required'),
    region: z.string().default('eu-central-1'),
    fromStage: z.string().min(1, 'From stage is required'),
    toStage: z.string().min(1, 'To stage is required'),
});

function getGithubConfig(repo) {
    const content = getGithubFile(repo, '.aws-changelog.json');

    if (!content) {
        consola.warn(
            `No .aws-changelog.json found in repository ${repo}. Using default configuration.`
        );
        return {};
    }

    try {
        const parsedConfig = JSON.parse(content);
        const validatedConfig = GithubConfigSchema.parse(parsedConfig);
        return validatedConfig;
    } catch (error) {
        consola.error(
            `Error parsing .aws-changelog.json in repository ${repo}: ${error.message}`
        );
        return {};
    }
}

export function parse(args) {
    consola.info(`Fetching repository configuration for ${args.repo}...`);

    const argsConfig = {
        pipeline: args.pipeline,
        region: args.region,
        fromStage: args.fromStage,
        toStage: args.toStage,
    };

    const githubConfig = !args.noGit ? getGithubConfig(args.repo) : {};

    const config = Object.assign(
        {
            pipeline: null,
            region: null,
            fromStage: null,
            toStage: null,
        },
        githubConfig,
        argsConfig
    );

    try {
        const validatedConfig = ConfigSchema.parse(config);
        return validatedConfig;
    } catch (error) {
        consola.error(`Invalid configuration: ${error.message}`);
        return null;
    }
}
