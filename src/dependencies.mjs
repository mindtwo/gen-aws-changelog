import consola from 'consola';
import { spawnSync } from 'node:child_process';

/**
 * Get env variable by name
 *
 * @param {*} name
 * @return {*}
 */
function requireEnv(name) {
    const value = process.env[name];
    if (!value) {
        return undefined;
    }

    return value;
}

/**
 * Check if gh cli is installed
 *
 * @return {boolean}
 */
function isGhInstalled() {
    const result = spawnSync('gh', ['--version'], {
        encoding: 'utf8',
    });

    // If command not found on most systems, error.code === 'ENOENT'
    if (result.error && result.error.code === 'ENOENT') {
        return false;
    }

    // Non-zero exit code also means something’s wrong
    if (result.status !== 0) {
        return false;
    }

    return true;
}

export function checkPrerequisites() {
    // Check if gh cli is installed
    if (!isGhInstalled()) {
        consola.error(
            'Github cli is not installed. Please install it first...'
        );
        return false;
    }

    // Check if Aws Credentials are in .env
    if (!requireEnv('AWS_ACCOUNT_ID')) {
        consola.error(
            'No aws credentials inside the current console environment found. Inject them first and rerun the command...'
        );
        return false;
    }

    return true;
}
