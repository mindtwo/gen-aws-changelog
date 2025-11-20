/**
 * Get error
 *
 * @export
 * @param {*} promise
 * @return {[Error|undefined, object]}
 */
export function catchError(promise) {
    return promise.then((data) => [undefined, data]).catch((error) => [error]);
}

export function catchErrorSync(callback) {
    try {
        const result = callback();

        return [undefined, result];
    } catch (error) {
        return [error, undefined];
    }
}

/**
 * Parse the repository name and split into owner + repository
 *
 * @export
 * @param {*} orgRepo
 * @return {*}
 */
export function parseReponame(orgRepo) {
    const parts = orgRepo.split('/');
    if (parts.length !== 2) {
        throw new Error(`Invalid repository format: ${orgRepo}`);
    }

    return {
        owner: parts[0],
        repository: parts[1],
    };
}
