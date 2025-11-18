// scripts/pipeline-commits.mjs
import {
    CodePipelineClient,
    GetPipelineStateCommand,
    GetPipelineExecutionCommand,
} from '@aws-sdk/client-codepipeline';
import { consola } from 'consola';

/**
 * Prints the commit information for a given stage.
 * @param {Object} commit - The commit information object.
 */
function printStageCommit(commit) {
    if (consola.level < 4) {
        // Only print if level is info or higher
        return;
    }

    consola.debug(`${commit.stage}:`);
    consola.debug('  Stage:', commit.stageName);
    consola.debug('  Execution ID:', commit.pipelineExecutionId);
    consola.debug('  Commit:', commit.revisionId);
    if (commit.revisionSummary) {
        consola.debug('  Summary:', commit.revisionSummary);
    }

    if (commit.revisionUrl) {
        consola.debug('  URL:', commit.revisionUrl);
    }
    consola.debug('');
}

export async function getAwsPipelineStages(config = {}) {
    const { fromStage, toStage, pipeline, region } = config;

    if (!fromStage || !toStage || !pipeline || !region) {
        throw new Error(
            'Configuration must include fromStage, toStage, pipeline'
        );
    }

    const client = new CodePipelineClient({ region });

    /**
     * Fetches the latest commit information for a given stage in the pipeline.
     * @param {string} stageName - The name of the stage to fetch commit information for.
     * @returns {Promise<Object>} An object containing the stage name, execution ID, and commit details.
     */
    async function getStageCommit(stageName) {
        const state = await client.send(
            new GetPipelineStateCommand({ name: pipeline })
        );

        const stage = state.stageStates?.find((s) => s.stageName === stageName);
        if (!stage) {
            throw new Error(
                `Stage "${stageName}" not found in pipeline "${pipeline}"`
            );
        }

        const latestExec = stage.latestExecution;
        if (!latestExec?.pipelineExecutionId) {
            throw new Error(`Stage "${stageName}" has no latestExecution`);
        }

        const pipelineExecutionId = latestExec.pipelineExecutionId;

        const execResp = await client.send(
            new GetPipelineExecutionCommand({
                pipelineName: pipeline,
                pipelineExecutionId,
            })
        );

        const artifact = execResp.pipelineExecution?.artifactRevisions?.[0];

        return {
            stage: stageName.replace('Deploy', ''),
            stageName,
            pipelineExecutionId,
            revisionId: artifact?.revisionId || null,
            revisionSummary: artifact?.revisionSummary || null,
            revisionUrl: artifact?.revisionUrl || null,
        };
    }

    try {
        const [preprod, prod] = await Promise.all(
            [fromStage, toStage].map((stageName) => getStageCommit(stageName))
        );

        consola.debug('=== Pipeline:', pipeline, '===\n');

        printStageCommit(preprod);
        printStageCommit(prod);

        return {
            fromStage: preprod,
            toStage: prod,
        };
    } catch (error) {
        consola.error('Error fetching pipeline commits:', error);
        return null;
    }
}
