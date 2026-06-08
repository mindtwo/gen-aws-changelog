#!/usr/bin/env node
// Check whether a list of S3 object keys exist in a selected bucket.
//
// Usage:
//   node src/scripts/s3-check-paths.mjs <paths-file>
//   node src/scripts/s3-check-paths.mjs <paths-file> --bucket my-bucket
//   node src/scripts/s3-check-paths.mjs <paths-file> --concurrency 20
//
// The paths file has one S3 key per line. Lines starting with '#' and empty
// lines are ignored. Leading 's3://bucket/' prefixes are stripped automatically.
//
// Credentials come from the standard AWS env vars / shared config
// (AWS_PROFILE, AWS_REGION, ~/.aws/credentials, ...).

import { argv, exit } from 'node:process';
import { checkS3Paths } from '../s3-check-paths.mjs';

function parseArgs(args) {
    const out = { file: null, bucket: null, concurrency: 10, showDeleted: false };
    for (let i = 0; i < args.length; i++) {
        const a = args[i];
        if (a === '--bucket') {
            out.bucket = args[++i];
        } else if (a === '--concurrency') {
            out.concurrency = Number.parseInt(args[++i], 10) || 10;
        } else if (a === '--show-deleted') {
            out.showDeleted = true;
        } else if (!out.file) {
            out.file = a;
        }
    }
    return out;
}

const opts = parseArgs(argv.slice(2));
if (!opts.file) {
    console.error('Usage: node src/scripts/s3-check-paths.mjs <paths-file> [--bucket name] [--concurrency 10] [--show-deleted]');
    exit(1);
}

try {
    const { errorCount } = await checkS3Paths(opts);
    exit(errorCount > 0 ? 2 : 0);
} catch (err) {
    if (err?.name === 'ExitPromptError') {
        exit(130);
    }
    console.error(err.message ?? err);
    exit(1);
}
