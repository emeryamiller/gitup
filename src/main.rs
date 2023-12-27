// use chrono::{DateTime, Utc};
use clap::Parser;
use core::fmt;
use regex::Regex;
use reqwest::header;
use reqwest::header::HeaderValue;
use serde::Deserialize;
use std::env;
use std::process::{exit, Command};
use std::str::from_utf8;
use url::Url;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about=None)]
struct Args {
    #[arg(short, long)]
    message: Option<String>,
    #[arg(short, long)]
    pull_request: bool,
}

#[derive(Debug)]
enum MessageKind {
    Feature,
    Fix,
    Chore,
}

#[derive(Debug)]
struct StoryId {
    team: String,
    id: i32,
}

#[derive(Debug)]
struct Message {
    kind: MessageKind,
    story: StoryId,
    body: String,
}

impl MessageKind {
    fn parse(kind: &str) -> MessageKind {
        match kind {
            "feat" => MessageKind::Feature,
            "fix" => MessageKind::Fix,
            "chore" => MessageKind::Chore,
            _ => panic!("Invalid message kind: {}", kind),
        }
    }
}

impl fmt::Display for MessageKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageKind::Feature => write!(f, "feat"),
            MessageKind::Fix => write!(f, "fix"),
            MessageKind::Chore => write!(f, "chore"),
        }
    }
}

impl StoryId {
    fn parse(story: &str) -> StoryId {
        let mut parts = story.split('-');
        let team = parts.next().unwrap();
        let id = parts.next().unwrap().parse::<i32>().unwrap();

        StoryId {
            team: team.to_string(),
            id,
        }
    }
}

impl fmt::Display for StoryId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}-{}", self.team, self.id)
    }
}

impl Message {
    fn parse(message: &str, branch: &str) -> Message {
        let regex = Regex::new(r"^(?<kind>\w+): *(?<story>\w+-\d+) +(?<body>.*)$")
            .expect("could not compile regex");
        match regex.captures(message) {
            Some(capture) => {
                let kind = capture.name("kind").unwrap();
                let story = capture.name("story").unwrap();
                let body = capture.name("body").unwrap();

                Message {
                    kind: MessageKind::parse(kind.as_str()),
                    story: StoryId::parse(story.as_str()),
                    body: body.as_str().to_string(),
                }
            }

            None => Message {
                kind: MessageKind::Feature,
                story: StoryId::parse(branch),
                body: message.to_string(),
            },
        }
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {} {}", self.kind, self.story, self.body)
    }
}

#[derive(Debug, Deserialize)]
struct PullRequest {
    // url: Url,
    html_url: Url,
    // diff_url: Url,
    // patch_url: Url,
    // merged_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
struct Repo {
    // url: Url,
    // html_url: Url,
    // title: String,
    // state: String,
    pull_request: PullRequest,
}

#[derive(Debug, Deserialize)]
struct GitResponse {
    // total_count: i32,
    // incomplete_results: bool,
    items: Vec<Repo>,
}

fn current_branch() -> String {
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
    println!("{}", output);
    output
        .lines()
        .find(|line| line.starts_with("https") && line.contains("/pull/"))
        .map(|line| line.trim().to_string())
}

fn git_amend(message: Option<Message>) {
    println!("Amending commit");
    Command::new("git")
        .arg("add")
        .arg(".")
        .output()
        .expect("failed to add");

    let mut amend_command = Command::new("git");
    amend_command.arg("commit").arg("--amend");

    match message {
        Some(msg) => amend_command.arg("-m").arg(msg.to_string()),
        None => amend_command.arg("--no-edit"),
    };

    amend_command.output().expect("failed to commit");

    Command::new("git")
        .arg("push")
        .arg("--force")
        .output()
        .expect("failed to push");
}

fn repo_name() -> Option<String> {
    let output = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output()
        .expect("failed to get pr list");

    let url_string = from_utf8(&output.stdout)
        .expect("failed to parse url")
        .trim()
        .replace(".git", "");

    if url_string.is_empty() {
        None
    } else {
        let url = Url::parse(&url_string).expect("failed to parse url");
        let mut path = url.path().to_string();
        path.remove(0);
        Some(path)
    }
}

fn pr_url_for(branch: String) -> Option<String> {
    let auth_value = format!("Bearer {}", env::var("GITHUB_TOKEN").unwrap());
    let mut auth_header_value = HeaderValue::from_str(&auth_value).unwrap();
    auth_header_value.set_sensitive(true);
    let mut headers = header::HeaderMap::new();
    headers.insert(header::ACCEPT, HeaderValue::from_static("application/json"));
    headers.insert(header::USER_AGENT, HeaderValue::from_static("gup"));
    headers.insert(header::HOST, HeaderValue::from_static("api.github.com"));
    headers.insert(
        "X-GitHub-Api-Version",
        HeaderValue::from_static("2022-11-28"),
    );
    headers.insert(header::AUTHORIZATION, auth_header_value);
    let client = reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap();

    let response: Option<GitResponse> = repo_name()
        .map(|name| {
            format!(
                "https://api.github.com/search/issues?q=repo:{}+is:pr+is:open+head:{}",
                name, branch
            )
        })
        .map(|url| client.get(url).send().unwrap().json().unwrap());

    println!("{:?}", response);
    return response
        .unwrap()
        .items
        .first()
        .map(|repo| repo.pull_request.html_url.to_string());
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args);

    let branch = current_branch();
    if branch == "main" || branch == "master" {
        eprintln!("You're on the master or main branch, you can't push to this branch");
        exit(1);
    }

    let message = args
        .message
        .as_ref()
        .map(|msg| Message::parse(msg, &branch));

    match is_remote_branch() {
        true => {
            git_amend(message);
            if args.pull_request {
                if let Some(url) = pr_url_for(branch) {
                    println!("Opening {}", url);
                    Command::new("open")
                        .arg(url)
                        .status()
                        .expect("failed to open browser");
                }
            }
        }
        false => {
            let Some(msg) = message else {
                eprintln!("You're on a local branch, you must provide a commit message");
                exit(1);
            };

            let pr_url = git_commit(msg.to_string());
            if let Some(url) = pr_url {
                Command::new("open")
                    .arg(url)
                    .output()
                    .expect("failed to open browser");
            }
        }
    }
    // TODO: attempt to read message type from the branch name as well (feat/story)
}
