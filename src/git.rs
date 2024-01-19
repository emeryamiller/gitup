use reqwest::header;
use reqwest::header::HeaderValue;
use serde::Deserialize;
use std::env;
use std::process::Command;
use std::str::from_utf8;
use url::Url;

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

pub fn current_branch() -> String {
    let output = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output()
        .expect("failed to get current branch");

    let branch = from_utf8(&output.stdout).expect("failed to parse current branch");

    branch.trim().to_string()
}

pub fn is_remote_branch() -> bool {
    let output = Command::new("git")
        .arg("status")
        .arg("-sb")
        .output()
        .expect("failed to get branch status");

    let first_line = from_utf8(output.stdout.split(|&c| c == b'\n').next().unwrap());

    first_line.is_ok_and(|line| line.starts_with("## ") && line.contains("..."))
}

pub fn git_commit(message: String) -> Option<String> {
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

pub fn git_amend() {
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

pub fn repo_name() -> Option<String> {
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

pub fn pr_url_for(branch: &str) -> Option<String> {
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

    return response
        .unwrap()
        .items
        .first()
        .map(|repo| repo.pull_request.html_url.to_string());
}

pub fn new_pr_url(branch: &str) -> String {
    format!(
        "https://github.com/{}/compare/{}?expand=1",
        repo_name().unwrap(),
        branch
    )
}
