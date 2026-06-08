use crate::error::Result;
use aws_sdk_s3::error::SdkError;
use aws_sdk_s3::operation::head_object::HeadObjectError;
use aws_sdk_s3::Client;

#[derive(Debug, Clone)]
pub struct ExistenceCheck {
    pub ok: bool,
    pub delete_marker: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct DeletionInfo {
    pub deleted_at: Option<String>,
    pub version_id: Option<String>,
}

pub struct S3 {
    client: Client,
}

impl S3 {
    pub fn new(sdk_config: &aws_config::SdkConfig) -> Self {
        Self {
            client: Client::new(sdk_config),
        }
    }

    pub async fn list_buckets(&self) -> Result<Vec<String>> {
        let out = self.client.list_buckets().send().await?;
        Ok(out
            .buckets
            .unwrap_or_default()
            .into_iter()
            .filter_map(|b| b.name)
            .collect())
    }

    pub async fn head(&self, bucket: &str, key: &str) -> ExistenceCheck {
        match self
            .client
            .head_object()
            .bucket(bucket)
            .key(key)
            .send()
            .await
        {
            Ok(out) => ExistenceCheck {
                ok: true,
                delete_marker: out.delete_marker().unwrap_or(false),
                error: None,
            },
            Err(SdkError::ServiceError(err)) => match err.err() {
                HeadObjectError::NotFound(_) => ExistenceCheck {
                    ok: false,
                    delete_marker: false,
                    error: None,
                },
                other => ExistenceCheck {
                    ok: false,
                    delete_marker: false,
                    error: Some(format!("{other}")),
                },
            },
            Err(err) => ExistenceCheck {
                ok: false,
                delete_marker: false,
                error: Some(format!("{err}")),
            },
        }
    }

    pub async fn deletion_info(&self, bucket: &str, key: &str) -> Result<Option<DeletionInfo>> {
        let resp = self
            .client
            .list_object_versions()
            .bucket(bucket)
            .prefix(key)
            .send()
            .await?;
        let marker = resp
            .delete_markers
            .unwrap_or_default()
            .into_iter()
            .find(|m| m.key.as_deref() == Some(key) && m.is_latest.unwrap_or(false));
        Ok(marker.map(|m| DeletionInfo {
            deleted_at: m.last_modified.map(|t| t.to_string()),
            version_id: m.version_id,
        }))
    }
}
