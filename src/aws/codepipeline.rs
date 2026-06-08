#![allow(dead_code)] // approval helpers consumed by Phase 6 (release)

use crate::error::{AppError, Result};
use aws_sdk_codepipeline::types::{ApprovalResult, ApprovalStatus, StageState};
use aws_sdk_codepipeline::Client;

/// Snapshot of one stage's latest deployed commit.
#[derive(Debug, Clone)]
pub struct StageRevision {
    pub stage: String,
    pub execution_id: String,
    pub revision_id: String,
    pub revision_url: Option<String>,
    pub revision_summary: Option<String>,
}

/// Identifies a pending manual-approval action so it can be approved later.
#[derive(Debug, Clone)]
pub struct PendingApproval {
    pub stage: String,
    pub action: String,
    pub token: String,
}

pub struct PipelineClient {
    client: Client,
    pipeline: String,
}

impl PipelineClient {
    pub fn new(sdk_config: &aws_config::SdkConfig, pipeline: impl Into<String>) -> Self {
        Self {
            client: Client::new(sdk_config),
            pipeline: pipeline.into(),
        }
    }

    pub async fn list_stages(&self) -> Result<Vec<String>> {
        let state = self
            .client
            .get_pipeline_state()
            .name(&self.pipeline)
            .send()
            .await?;
        Ok(state
            .stage_states
            .unwrap_or_default()
            .into_iter()
            .filter_map(|s| s.stage_name)
            .collect())
    }

    pub async fn stage_revision(&self, stage_name: &str) -> Result<StageRevision> {
        let state = self
            .client
            .get_pipeline_state()
            .name(&self.pipeline)
            .send()
            .await?;
        let stages = state.stage_states.unwrap_or_default();
        let stage = stages
            .into_iter()
            .find(|s| s.stage_name.as_deref() == Some(stage_name))
            .ok_or_else(|| {
                AppError::StageNotFound(stage_name.to_string(), self.pipeline.clone())
            })?;

        let execution_id = stage
            .latest_execution
            .as_ref()
            .map(|e| e.pipeline_execution_id.clone())
            .ok_or_else(|| {
                anyhow::anyhow!("stage '{stage_name}' has no latest execution")
            })?;

        let execution = self
            .client
            .get_pipeline_execution()
            .pipeline_name(&self.pipeline)
            .pipeline_execution_id(&execution_id)
            .send()
            .await?
            .pipeline_execution
            .ok_or_else(|| {
                anyhow::anyhow!("no pipeline execution returned for {execution_id}")
            })?;

        let revision = execution
            .artifact_revisions()
            .first()
            .ok_or_else(|| {
                anyhow::anyhow!("execution {execution_id} has no artifact revisions")
            })?
            .clone();

        Ok(StageRevision {
            stage: stage_name.to_string(),
            execution_id,
            revision_id: revision
                .revision_id
                .ok_or_else(|| anyhow::anyhow!("revision had no id"))?,
            revision_url: revision.revision_url,
            revision_summary: revision.revision_summary,
        })
    }

    /// Walk the stage's actions and return the first one in `InProgress`
    /// state belonging to a manual approval category.
    pub async fn pending_approval(&self, stage_name: &str) -> Result<PendingApproval> {
        let state = self
            .client
            .get_pipeline_state()
            .name(&self.pipeline)
            .send()
            .await?;
        let stage = state
            .stage_states
            .unwrap_or_default()
            .into_iter()
            .find(|s| s.stage_name.as_deref() == Some(stage_name))
            .ok_or_else(|| {
                AppError::StageNotFound(stage_name.to_string(), self.pipeline.clone())
            })?;

        find_pending_approval(&stage)
            .ok_or_else(|| AppError::NoPendingApproval(stage_name.to_string()).into())
    }

    pub async fn approve(
        &self,
        stage: &str,
        action: &str,
        token: &str,
        summary: &str,
    ) -> Result<()> {
        let result = ApprovalResult::builder()
            .status(ApprovalStatus::Approved)
            .summary(summary)
            .build()?;
        self.client
            .put_approval_result()
            .pipeline_name(&self.pipeline)
            .stage_name(stage)
            .action_name(action)
            .token(token)
            .result(result)
            .send()
            .await?;
        Ok(())
    }
}

fn find_pending_approval(stage: &StageState) -> Option<PendingApproval> {
    let stage_name = stage.stage_name.clone()?;
    for action in stage.action_states() {
        let latest = action.latest_execution.as_ref()?;
        if !matches!(
            latest.status,
            Some(aws_sdk_codepipeline::types::ActionExecutionStatus::InProgress)
        ) {
            continue;
        }
        let token = latest.token.clone()?;
        return Some(PendingApproval {
            stage: stage_name,
            action: action.action_name.clone()?,
            token,
        });
    }
    None
}
