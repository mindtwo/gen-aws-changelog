import { execFileSync } from 'node:child_process';
import { consola } from 'consola';

/**
 * Generates a changelog between two commits in a GitHub repository.
 * @param {string} toSha - The SHA of the commit to compare from.
 * @param {string} fromSha - The SHA of the commit to compare to.
 * @returns {string} A formatted changelog string.
 */
export function parseReponame(orgRepo) {
    const parts = orgRepo.split('/');
    if (parts.length !== 2) {
        throw new Error(`Invalid repository format: ${orgRepo}`);
    }

    return {
        organization: parts[0],
        repository: parts[1],
    };
}

export function getGithubFile(orgRepo, path, printError = false) {
    const { organization, repository } =
        typeof orgRepo === 'string' ? parseReponame(orgRepo) : orgRepo;

    // Use gh CLI to get the file content in base64
    // gh api repos/{organization}/{repository}/contents/{path} -f ref={ref} --jq '.content'
    try {
        const response = execFileSync(
            'gh',
            ['api', `repos/${organization}/${repository}/contents/${path}`],
            { encoding: 'utf8' }
        );

        const data = JSON.parse(response);

        if (!data.content) {
            return null;
        }

        // decode base64 in JS
        return Buffer.from(response.content.trim(), 'base64').toString('utf8');
    } catch (error) {
        if (printError) {
            consola.error(
                `Error fetching file ${path} from repository ${orgRepo}:`,
                error.message
            );
        }
        return null;
    }
}

/**
 * Compares two commits in a GitHub repository and returns the commit messages.
 * @param {string} orgRepo - The repository in the format 'organization/repository'.
 * @param {string} toSha - The SHA of the commit to compare from.
 * @param {string} fromSha - The SHA of the commit to compare to.
 * @returns {string[]} An array of commit messages.
 */
export function compareCommits(orgRepo, toSha, fromSha) {
    const { organization, repository } =
        typeof orgRepo === 'string' ? parseReponame(orgRepo) : orgRepo;

    const jqExpr =
        '.commits[] | "- \\(.sha[0:7]) \\(.commit.message | split("\\n")[0])"';

    return execFileSync(
        'gh',
        [
            'api',
            `repos/${organization}/${repository}/compare/${toSha}...${fromSha}`,
            '--jq',
            jqExpr,
        ],
        { encoding: 'utf8' }
    );
}
