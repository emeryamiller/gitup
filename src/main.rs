mod commit_message;
mod git;

use clap::Parser;
use commit_message::{Message, MessageKind};
use git::{current_branch, git_amend, git_commit, is_remote_branch, new_pr_url, pr_url_for};
use std::fmt::Display;
use std::process::{exit, Command};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    message: Option<String>,
    #[arg(short, long)]
    pull_request: bool,
    #[arg(short, long)]
    chore: bool,
    #[arg(short = 'x', long)]
    fix: bool,
    #[arg(short, long)]
    ignore_format: bool,
}

fn open_url(url: &str) {
    Command::new("open")
        .arg(url)
        .status()
        .expect("failed to open browser");
}

fn compose_message(args: &Args, branch: &str) -> Option<Box<dyn Display>> {
    if args.ignore_format {
        return args
            .message
            .as_ref()
            .map(|msg| Box::new(msg.clone()) as Box<dyn Display>);
    }

    let kind = if args.chore {
        MessageKind::Chore
    } else if args.fix {
        MessageKind::Fix
    } else {
        MessageKind::Feature
    };

    args.message.as_ref().map(|msg| {
        let parsed_message = Message::parse(msg, branch, Some(kind));
        if let Ok(message) = parsed_message {
            Box::new(message) as Box<dyn Display>
        } else {
            let new_message = edit::edit(msg).expect("failed to edit message");
            Box::new(Message::parse(&new_message, branch, Some(kind)).unwrap()) as Box<dyn Display>
        }
    })
}

fn main() {
    let args = Args::parse();
    let branch = current_branch();
    if branch == "main" || branch == "master" {
        eprintln!("You're on the master or main branch, you can't push to this branch");
        exit(1);
    }
    let message = compose_message(&args, &branch);

    match is_remote_branch() {
        true => {
            match message {
                Some(msg) => {
                    git_commit(msg.to_string());
                }
                None => git_amend(),
            };
            if args.pull_request {
                if let Some(url) = pr_url_for(&branch) {
                    println!("Opening {}", url);
                    open_url(url.as_str());
                }
            }
        }
        false => {
            let msg = match message {
                Some(m) => m,
                None => {
                    let new_message = edit::edit("").expect("failed to edit message");
                    if new_message.is_empty() {
                        eprintln!("You're on a local branch, you must provide a commit message");
                        exit(1);
                    }
                    Box::new(Message::parse(&new_message, &branch, None).unwrap())
                }
            };

            let pr_url = git_commit(msg.to_string());
            if let Some(url) = pr_url {
                open_url(&url);
            } else {
                open_url(&new_pr_url(&branch));
            }
        }
    }
    // Second tool, poll for a period until PR passes all checks, then report back to slack
}
