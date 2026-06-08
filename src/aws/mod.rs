//! Thin wrappers around the AWS SDKs used across commands. The shared
//! [`load_sdk_config`] helper enforces a region override and reuses the
//! default credential provider chain (env vars, shared credentials file,
//! SSO, IMDS, ...).

pub mod codepipeline;
pub mod s3;

use aws_config::{BehaviorVersion, Region, SdkConfig};

pub async fn load_sdk_config(region: &str) -> SdkConfig {
    aws_config::defaults(BehaviorVersion::latest())
        .region(Region::new(region.to_string()))
        .load()
        .await
}
