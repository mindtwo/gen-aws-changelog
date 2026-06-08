import { readFile } from 'node:fs/promises';
import { stdout } from 'node:process';
import {
    S3Client,
    ListBucketsCommand,
    HeadObjectCommand,
    ListObjectVersionsCommand,
} from '@aws-sdk/client-s3';
import { search } from '@inquirer/prompts';

function normalizeKey(line) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith('#')) {
        return null;
    }
    if (trimmed.startsWith('s3://')) {
        const without = trimmed.slice('s3://'.length);
        const slash = without.indexOf('/');
        return slash === -1 ? '' : without.slice(slash + 1);
    }
    return trimmed.replace(/^\/+/, '');
}

async function pickBucket(client) {
    const { Buckets = [] } = await client.send(new ListBucketsCommand({}));
    if (Buckets.length === 0) {
        throw new Error('No S3 buckets visible with the current credentials.');
    }
    const choices = Buckets.map((b) => ({ name: b.Name, value: b.Name }));
    return search({
        message: 'Select an S3 bucket',
        source: async (term) => {
            if (!term) return choices;
            const needle = term.toLowerCase();
            return choices.filter((c) => c.name.toLowerCase().includes(needle));
        },
    });
}

async function exists(client, bucket, key) {
    try {
        const res = await client.send(
            new HeadObjectCommand({ Bucket: bucket, Key: key })
        );
        return { ok: true, deleteMarker: res.DeleteMarker === true };
    } catch (err) {
        const status = err?.$metadata?.httpStatusCode;
        const headers = err?.$response?.headers ?? err?.$metadata?.httpHeaders;
        const deleteMarker = headers?.['x-amz-delete-marker'] === 'true';
        if (status === 404 || err?.name === 'NotFound') {
            return { ok: false, deleteMarker };
        }
        return { ok: false, error: err?.name ?? err?.message ?? 'UnknownError' };
    }
}

async function findDeletionInfo(client, bucket, key) {
    const res = await client.send(
        new ListObjectVersionsCommand({
            Bucket: bucket,
            Prefix: key,
            MaxKeys: 100,
        })
    );

    const markers = (res.DeleteMarkers ?? []).filter((m) => m.Key === key);
    if (markers.length === 0) {
        return null;
    }

    const latest =
        markers.find((m) => m.IsLatest) ??
        markers.sort((a, b) => b.LastModified - a.LastModified)[0];

    return {
        deletedAt: latest.LastModified,
        versionId: latest.VersionId,
    };
}

async function runWithConcurrency(items, limit, worker) {
    const results = new Array(items.length);
    let next = 0;
    const runners = Array.from({ length: Math.min(limit, items.length) }, async () => {
        while (true) {
            const i = next++;
            if (i >= items.length) return;
            results[i] = await worker(items[i], i);
        }
    });
    await Promise.all(runners);
    return results;
}

export async function checkS3Paths({
    file,
    bucket: bucketArg,
    concurrency = 10,
    showDeleted = false,
}) {
    if (!file) {
        throw new Error('A paths file is required.');
    }

    const raw = await readFile(file, 'utf8');
    const keys = raw
        .split(/\r?\n/)
        .map((l) => normalizeKey(l))
        .filter((k) => k !== null && k !== '');

    if (keys.length === 0) {
        throw new Error(`No usable paths found in ${file}.`);
    }

    const client = new S3Client({});
    const bucket = bucketArg ?? (await pickBucket(client));

    stdout.write(`\nChecking ${keys.length} path(s) in bucket "${bucket}"...\n\n`);

    let existCount = 0;
    let missingCount = 0;
    let deletedCount = 0;
    let errorCount = 0;

    await runWithConcurrency(keys, concurrency, async (key) => {
        const result = await exists(client, bucket, key);
        if (result.error) {
            errorCount++;
            stdout.write(`error      ${key}  (${result.error})\n`);
            return;
        }
        if (result.ok) {
            existCount++;
            stdout.write(`exists     ${key}\n`);
            return;
        }

        if (!showDeleted) {
            missingCount++;
            stdout.write(`not exists ${key}\n`);
            return;
        }

        try {
            const info = await findDeletionInfo(client, bucket, key);
            if (info) {
                deletedCount++;
                stdout.write(
                    `deleted    ${key}  (at ${info.deletedAt.toISOString()}, versionId=${info.versionId})\n`
                );
            } else {
                missingCount++;
                stdout.write(`not exists ${key}\n`);
            }
        } catch (err) {
            errorCount++;
            const reason = err?.name ?? err?.message ?? 'UnknownError';
            stdout.write(`error      ${key}  (versions: ${reason})\n`);
        }
    });

    const summary = showDeleted
        ? `exists=${existCount}  deleted=${deletedCount}  not exists=${missingCount}  errors=${errorCount}`
        : `exists=${existCount}  not exists=${missingCount}  errors=${errorCount}`;
    stdout.write(`\nDone. ${summary}\n`);

    return { existCount, missingCount, deletedCount, errorCount };
}
