mod prompt;

use git_commitizen::{
    build_commit_message, build_commit_types, format_commit_types, perform_commit,
};
use std::env;
use std::path::Path;
use std::process::Command;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let commit_types_display = format_commit_types(build_commit_types());
    let selection = prompt::select(
        "Select the type of change that you're committing:",
        &commit_types_display,
        10,
    )
    .await?;
    let selected_type = selection.split_whitespace().next();

    if let Some(commit_type) = selected_type {
        let scope = prompt::input("Denote the scope of this change (optional):").await?;
        let description =
            prompt::input("Write a short, imperative tense description of the change:").await?;
        let body =
            prompt::input("Provide a longer description of the change (press 'e' to open editor):")
                .await?;

        let body = if body.trim().to_lowercase() == "e" {
            let temp_file = tempfile::NamedTempFile::new()?;
            let editor_command = if cfg!(target_os = "windows") {
                env::var("EDITOR").unwrap_or_else(|_| "notepad".to_string())
            } else {
                env::var("EDITOR").unwrap_or_else(|_| "vim".to_string())
            };

            let status = Command::new(&editor_command)
                .arg(temp_file.path())
                .status()?;

            if !status.success() {
                eprintln!("Editor exited with non-zero status");
            }

            std::fs::read_to_string(temp_file.path())?
        } else {
            body
        };

        let footer_choice = prompt::select(
            "Do you want to add a footer?",
            &["No".to_string(), "Yes".to_string()],
            2,
        )
        .await?;
        let footer = if footer_choice == "Yes" {
            let footer_type = prompt::select(
                "Select the footer type:",
                &["fix".to_string(), "close".to_string()],
                2,
            )
            .await?;
            let issue_number = prompt::integer("Enter the issue number:").await?;
            format!("{}: #{}", footer_type, issue_number)
        } else {
            String::new()
        };

        let full_commit_message =
            build_commit_message(commit_type, &scope, &description, &body, &footer);

        let confirm = prompt::select(
            "Do you want to proceed with this commit?",
            &["Yes".to_string(), "No".to_string()],
            2,
        )
        .await?;
        if confirm == "Yes" {
            perform_commit(Path::new("."), &full_commit_message)?;
            println!("Commit successful!");
        } else {
            println!("Commit aborted.");
        }
    }

    Ok(())
}
