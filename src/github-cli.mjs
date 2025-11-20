import { spawnSync } from 'node:child_process';
import { catchErrorSync, parseReponame } from './util.mjs';

/**
 * Get data safely
 *
 * @param {*} args
 * @return {object|string}
 */
function safeGhApi(args) {
    const result = spawnSync('gh', ['api', ...args], {
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'], // don't inherit stderr
    });

    if (result.status !== 0) {
        // result.stderr contains "gh: Not Found (HTTP 404)" etc.
        const [error, data] = catchErrorSync(() =>
            result.stderr ? JSON.parse(result.stderr) : null
        );

        return data ?? result?.stderr;
    }

    return JSON.parse(result.stdout);
}

/**
 * Parse commit messages into format:
 * "- \\(.sha[0:7]) \\(.commit.message | split("\\n")[0])"
 *
 * @param {object[]} commits
 * @returns {string[]} An array of commit messages.
 */
function defaultCommitsParser(commits) {
    return commits.map((commit) => {
        const sha = commit.sha.slice(0, 7);
        const message = (commit.commit?.message ?? '').split('\n')[0];

        return `- ${sha} ${message}`;
    });
}

/**
 * Get file contents from repository
 *
 * @export
 * @param {*} orgRepo
 * @param {*} path
 * @return {*}
 */
export function getGithubFile(orgRepo, path) {
    const { owner, repository } =
        typeof orgRepo === 'string' ? parseReponame(orgRepo) : orgRepo;

    const data = safeGhApi([`repos/${owner}/${repository}/contents/${path}`]);
    if (!data?.content) {
        return null;
    }

    const contentBase64 = data.content;

    return Buffer.from(contentBase64.trim(), 'base64').toString('utf8');
}

/**
 * Compares two commits in a GitHub repository and returns the commit messages.
 * @param {string} orgRepo - The repository in the format 'owner/repository'.
 * @param {string} toSha - The SHA of the commit to compare from.
 * @param {string} fromSha - The SHA of the commit to compare to.
 * @returns {string[]} An array of commit messages.
 */
export function compareCommits(orgRepo, toSha, fromSha) {
    const { owner, repository } =
        typeof orgRepo === 'string' ? parseReponame(orgRepo) : orgRepo;

    const data = safeGhApi([
        `repos/${owner}/${repository}/compare/${toSha}...${fromSha}`,
    ]);

    if (!data.commits) {
        return [];
    }

    return defaultCommitsParser(data.commits);
}

export function createRelease(tag, title, changelog, repo) {
    const result = spawnSync(
        'gh',
        [
            'release',
            'create',
            tag,
            '--title',
            title,
            '--notes',
            changelog,
            '--repo',
            repo,
        ],
        {
            encoding: 'utf8',
            stdio: ['ignore', 'pipe', 'pipe'], // don't inherit stderr
        }
    );

    return result.stdout;
}
