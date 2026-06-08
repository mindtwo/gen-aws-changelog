use crate::aws::{load_sdk_config, s3::S3};
use crate::cli::S3CheckArgs;
use crate::error::Result;
use crate::ui::{progress, prompts};
use colored::Colorize;
use futures::stream::{FuturesUnordered, StreamExt};
use std::sync::Arc;

const DEFAULT_REGION: &str = "eu-central-1";

pub async fn run(args: S3CheckArgs) -> Result<()> {
    let keys = read_keys(&args.file)?;
    if keys.is_empty() {
        anyhow::bail!("no keys in {}", args.file.display());
    }

    let region = std::env::var("AWS_REGION").unwrap_or_else(|_| DEFAULT_REGION.to_string());
    let sdk = load_sdk_config(&region).await;
    let s3 = Arc::new(S3::new(&sdk));

    let bucket = match args.bucket {
        Some(b) => b,
        None => pick_bucket(&s3).await?,
    };

    println!(
        "Checking {} path(s) in bucket {}...\n",
        keys.len(),
        bucket.bold()
    );

    let pb = progress::bar(keys.len() as u64, "checking");
    let mut results: Vec<KeyResult> = Vec::with_capacity(keys.len());
    let mut tasks = FuturesUnordered::new();

    let mut iter = keys.into_iter();

    // Prime the concurrency window.
    for _ in 0..args.concurrency {
        if let Some(key) = iter.next() {
            let s3 = s3.clone();
            let bucket = bucket.clone();
            tasks.push(tokio::spawn(async move {
                let check = s3.head(&bucket, &key).await;
                (key, check)
            }));
        }
    }

    while let Some(joined) = tasks.next().await {
        let (key, check) = joined?;
        pb.inc(1);
        results.push(KeyResult { key, check });

        if let Some(next) = iter.next() {
            let s3 = s3.clone();
            let bucket = bucket.clone();
            tasks.push(tokio::spawn(async move {
                let check = s3.head(&bucket, &next).await;
                (next, check)
            }));
        }
    }
    pb.finish_and_clear();

    let mut exists = 0usize;
    let mut missing = 0usize;
    let mut deleted = 0usize;
    let mut errors = 0usize;

    for r in &results {
        if let Some(err) = &r.check.error {
            errors += 1;
            println!("{:<10} {}  ({})", "error".red().bold(), r.key, err);
        } else if r.check.ok {
            if r.check.delete_marker {
                deleted += 1;
                println!("{:<10} {}", "deleted".magenta().bold(), r.key);
            } else {
                exists += 1;
                println!("{:<10} {}", "exists".green().bold(), r.key);
            }
        } else {
            missing += 1;
            println!("{:<10} {}", "not exists".yellow().bold(), r.key);
        }
    }

    if args.show_deleted && missing > 0 {
        println!("\n{}", "Looking up delete markers for missing keys...".dimmed());
        for r in &results {
            if !r.check.ok && r.check.error.is_none() {
                if let Ok(Some(info)) = s3.deletion_info(&bucket, &r.key).await {
                    println!(
                        "  {} deleted at {} (version {})",
                        r.key,
                        info.deleted_at.unwrap_or_else(|| "?".into()),
                        info.version_id.unwrap_or_else(|| "?".into())
                    );
                }
            }
        }
    }

    println!(
        "\n{} {} exists  {} missing  {} deleted  {} errors",
        "summary:".bold(),
        exists,
        missing,
        deleted,
        errors,
    );

    if errors > 0 {
        std::process::exit(2);
    }
    Ok(())
}

struct KeyResult {
    key: String,
    check: crate::aws::s3::ExistenceCheck,
}

fn read_keys(path: &std::path::Path) -> Result<Vec<String>> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for raw in text.lines() {
        let line = raw.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let stripped = line
            .trim_start_matches("s3://")
            .trim_start_matches('/')
            .to_string();
        // For `s3://bucket/key` lines, drop the bucket prefix.
        let cleaned = if line.starts_with("s3://") {
            stripped.splitn(2, '/').nth(1).unwrap_or(&stripped).to_string()
        } else {
            stripped
        };
        if !cleaned.is_empty() {
            out.push(cleaned);
        }
    }
    Ok(out)
}

async fn pick_bucket(s3: &S3) -> Result<String> {
    let buckets = s3.list_buckets().await?;
    if buckets.is_empty() {
        anyhow::bail!("no S3 buckets visible to the current AWS credentials");
    }
    let idx = prompts::select("Select a bucket", &buckets)?;
    Ok(buckets[idx].clone())
}

#[cfg(test)]
mod tests {
    use super::read_keys;
    use std::io::Write;

    #[test]
    fn parses_basic_keys() {
        let mut tmp = tempfile::NamedTempFile::new().unwrap();
        writeln!(tmp, "# comment\nfoo/bar\n/leading/slash\ns3://mybucket/some/key\n  whitespaced  ").unwrap();
        let keys = read_keys(tmp.path()).unwrap();
        assert_eq!(
            keys,
            vec![
                "foo/bar".to_string(),
                "leading/slash".to_string(),
                "some/key".to_string(),
                "whitespaced".to_string(),
            ]
        );
    }
}
