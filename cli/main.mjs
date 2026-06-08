import { defineCommand, runMain } from 'citty';
import changelog from './commands/changelog.mjs';
import s3CheckPaths from './commands/s3-check-paths.mjs';

const main = defineCommand({
    meta: {
        name: 'gen-aws-changelog',
        version: '1.0.0',
        description: 'Generate AWS changelog from pipeline commits',
    },
    subCommands: {
        changelog,
        's3-check-paths': s3CheckPaths,
    },
});

runMain(main);
