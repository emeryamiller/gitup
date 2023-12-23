use clap::{Arg, Parser};
use std::process::{exit, Command};
use std::str::from_utf8;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    message: Option<String>,
    #[arg(short, long)]
    pull_request: bool,
}

fn get_current_branch() -> String {
    let output = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        .expect("failed to get current branch");

    let branch = from_utf8(&output.stdout).expect("failed to parse current branch");

    branch.trim().to_string()
}

fn is_remote_branch() -> bool {
    let output = Command::new("git")
        .arg("status")
        .arg("-sb")
        .output()
        .expect("failed to get branch status");

    let first_line = from_utf8(output.stdout.split(|&c| c == b'\n').next().unwrap());

    first_line.is_ok_and(|line| line.starts_with("## ") && line.contains("..."))
}

fn git_commit(message: String) -> Option<String> {
    println!("Commiting commit");
    Command::new("git")
        .arg("add")
        .arg(".")
        .output()
        .expect("failed to add");

    Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .output()
        .expect("failed to commit");

    let command = Command::new("git")
        .arg("push")
        .output()
        .expect("failed to push");

    let output = from_utf8(&command.stdout).expect("failed to parse push output");
    output
        .lines()
        .find(|line| line.starts_with("https") && line.contains("/pull/"))
        .map(|line| line.trim().to_string())
}

fn git_amend() {
    println!("Amending commit");
    Command::new("git")
        .arg("add")
        .arg(".")
        .output()
        .expect("failed to add");

    Command::new("git")
        .arg("commit")
        .arg("--amend")
        .arg("--no-edit")
        .output()
        .expect("failed to commit");

    Command::new("git")
        .arg("push")
        .arg("--force")
        .output()
        .expect("failed to push");
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);

    let branch = get_current_branch();
    if branch == "main" || branch == "master" {
        eprintln!("You're on the master or main branch, you can't push to this branch");
        exit(1);
    }

    match is_remote_branch() {
        true => git_amend(),
        false => {
            let pr_url = git_commit(args.message.unwrap());
            if let Some(url) = pr_url {
                Command::new("open")
                    .arg(url)
                    .output()
                    .expect("failed to open browser");
            }
        }
    }
    // Can override the no-edit with a message
    // Maybe use a flag to open url?
}
