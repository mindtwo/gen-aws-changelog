use crate::aws::codepipeline::StageRevision;
use prettytable::{format, row, Table};

pub fn print_stage_revisions(pipeline: &str, from: &StageRevision, to: &StageRevision) {
    println!("pipeline: {pipeline}\n");
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_LINESEP_WITH_TITLE);
    table.set_titles(row!["stage", "revision", "summary"]);
    table.add_row(row![
        from.stage,
        short_sha(&from.revision_id),
        from.revision_summary.clone().unwrap_or_default(),
    ]);
    table.add_row(row![
        to.stage,
        short_sha(&to.revision_id),
        to.revision_summary.clone().unwrap_or_default(),
    ]);
    table.printstd();
}

pub fn short_sha(sha: &str) -> String {
    sha.chars().take(7).collect()
}
