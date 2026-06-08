import { defineCommand } from 'citty';
import consola from 'consola';
import { checkS3Paths } from '../../src/s3-check-paths.mjs';

export default defineCommand({
    meta: {
        name: 's3-check-paths',
        description:
            'Check whether a list of S3 object keys exist in a selected bucket',
    },
    args: {
        file: {
            type: 'positional',
            description: 'Path to a file containing one S3 key per line',
            required: true,
        },
        bucket: {
            type: 'string',
            description:
                'S3 bucket name. If omitted, an interactive picker is shown.',
        },
        concurrency: {
            type: 'string',
            description: 'Number of parallel HEAD requests (default: 10)',
            default: '10',
        },
        showDeleted: {
            type: 'boolean',
            description:
                'For missing keys, look up delete markers (requires bucket versioning) and report when they were deleted',
            default: false,
        },
    },
    async run({ args }) {
        const concurrency = Number.parseInt(args.concurrency, 10) || 10;

        try {
            const { errorCount } = await checkS3Paths({
                file: args.file,
                bucket: args.bucket,
                concurrency,
                showDeleted: args.showDeleted,
            });

            process.exit(errorCount > 0 ? 2 : 0);
        } catch (err) {
            if (err?.name === 'ExitPromptError') {
                process.exit(130);
            }
            consola.error(err.message ?? err);
            process.exit(1);
        }
    },
});
