use indicatif::{ProgressBar, ProgressStyle};

pub fn bar(total: u64, label: &str) -> ProgressBar {
    let pb = ProgressBar::new(total);
    pb.set_style(
        ProgressStyle::with_template(
            "{prefix:>12.cyan.bold} [{bar:32}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    pb.set_prefix(label.to_string());
    pb
}
