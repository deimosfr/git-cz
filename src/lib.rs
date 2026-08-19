use git2::Repository;
use std::error::Error;
use std::path::Path;
use std::process::Command;

pub fn build_commit_types() -> Vec<(&'static str, &'static str)> {
    vec![
        ("feat", "A new feature"),
        ("fix", "A bug fix"),
        ("docs", "Documentation only changes"),
        (
            "style",
            "Changes that do not affect the meaning of the code (white-space, formatting, etc.)",
        ),
        (
            "refactor",
            "A code change that neither fixes a bug nor adds a feature",
        ),
        ("perf", "A code change that improves performance"),
        ("test", "Adding missing tests or correcting existing tests"),
        ("chore", "Other changes that don't modify src or test files"),
        ("ci", "Changes to our CI configuration files and scripts"),
        (
            "build",
            "Changes that affect the build system or external dependencies",
        ),
        ("revert", "Reverts a previous commit"),
    ]
}

pub fn format_commit_types(commit_types: Vec<(&str, &str)>) -> Vec<String> {
    // Determine the maximum length of commit type strings for proper alignment
    let max_type_length = commit_types
        .iter()
        .map(|(typ, _)| typ.len())
        .max()
        .unwrap_or(0);

    commit_types
        .iter()
        .map(|(typ, desc)| {
            // Adjust the width to account for proper spacing
            format!("{:<width$} - {}", typ, desc, width = max_type_length + 4)
        })
        .collect()
}

pub fn build_commit_message(
    commit_type: &str,
    scope: &str,
    description: &str,
    body: &str,
    footer: &str,
) -> String {
    let message = format!(
        "{}{}: {}",
        commit_type,
        if scope.is_empty() {
            String::new()
        } else {
            format!("({})", scope)
        },
        description
    );

    let mut full_message = message;

    if !body.is_empty() {
        full_message.push_str(&format!("\n\n{}", body));
    }

    if !footer.is_empty() {
        full_message.push_str(&format!("\n\n{}", footer));
    }

    full_message
}

pub fn perform_commit(repo_path: &Path, full_commit_message: &str) -> Result<(), Box<dyn Error>> {
    let repo = Repository::discover(repo_path)?;

    let statuses = repo.statuses(None)?;
    if statuses.is_empty() {
        return Err("Nothing to commit, working directory clean".into());
    }

    let status = Command::new("git")
        .args(["commit", "-m", full_commit_message])
        .current_dir(repo_path)
        .status()?;

    if !status.success() {
        return Err(format!("git commit failed with status {status}").into());
    }

    Ok(())
}
