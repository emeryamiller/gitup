mod git;
pub use git::*;

use reqwest::blocking::Response;
use reqwest::header;
use reqwest::header::HeaderValue;
use reqwest::Result;
use serde::Deserialize;
use std::env;
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

#[derive(Debug, Deserialize)]
struct GitCheckRunResponse {
    // totol_count: i32,
    check_runs: Vec<GitCheckRun>,
}

#[derive(Debug, Deserialize)]
struct GitCheckRun {
    name: String,
    status: String,
    conclusion: String,
}

#[derive(Debug, Deserialize)]
struct GitPullsResponse {
    items: Vec<GitPull>,
}

#[derive(Debug, Deserialize)]
pub struct GitPull {
    pub html_url: String,
    pub title: String,
    pub state: String,
    pub draft: bool,
}

const GITHUB_TOKEN: &str = "GITHUB_TOKEN";
const GITHUB_TOKEN_MISSING: i32 = 20;
const REPO_NAME_NOT_FOUND: i32 = 5;

fn github_client() -> reqwest::blocking::Client {
    let auth_value = format!(
        "Bearer {}",
        env::var(GITHUB_TOKEN).unwrap_or_else(|err| exit_with_error(
            &format!("Please set {}: {}", GITHUB_TOKEN, err),
            GITHUB_TOKEN_MISSING
        ))
    );
    let mut auth_header_value = HeaderValue::from_str(&auth_value)
        .expect("Failed to create auth header - invalid characters.");
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

    reqwest::blocking::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap()
}

pub fn pr_url_for(branch: &str) -> Option<String> {
    let client = github_client();
    let response: Option<GitResponse> = repo_name()
        .map(|name| {
            format!(
                "https://api.github.com/search/issues?q=repo:{}+is:pr+is:open+head:{}",
                name, branch
            )
        })
        .and_then(|url| extract_json_or_none(client.get(url).send()));

    response.and_then(|res| {
        res.items
            .first()
            .and_then(|repo| Some(repo.pull_request.html_url.to_string()))
    })
}

pub fn new_pr_url(branch: &str) -> String {
    format!(
        "https://github.com/{}/compare/{}?expand=1",
        repo_name().unwrap_or_else(|| exit_with_error(
            &format!("Could not find repo name for branch {}", branch),
            REPO_NAME_NOT_FOUND
        )),
        branch
    )
}

pub fn commit_status() -> Option<bool> {
    let url = remote_branch_name()
        .and_then(|branch| remote_commit_id(&branch))
        .map(|id| {
            format!(
                "https://api.github.com/repos/{}/commits/{}/check-runs",
                repo_name().unwrap(),
                id
            )
        });

    if url.is_none() {
        panic!("Failed to find last remote commit id");
    }

    let client = github_client();
    let response: GitCheckRunResponse = client.get(url.unwrap()).send().unwrap().json().unwrap();
    if response
        .check_runs
        .iter()
        .any(|run| !run.status.eq("completed"))
    {
        return None;
    }

    Some(
        response
            .check_runs
            .iter()
            .all(|run| run.conclusion.eq("success") || run.name.eq("SonarQube Code Analysis")),
    )
}

pub fn branch_pr() -> Option<GitPull> {
    let client = github_client();
    let response: Option<GitPullsResponse> = remote_branch_name()
        .and_then(|branch| remote_commit_id(&branch))
        .map(|id| {
            format!(
                "https://api.github.com/repos/{}/commits/{}/pulls",
                repo_name().unwrap(),
                id
            )
        })
        .and_then(|url| client.get(url).send().unwrap().json().unwrap_or(None));

    response.and_then(|prs| prs.items.into_iter().find(|pr| pr.state.eq("open")))
}

fn extract_json_or_none(response: Result<Response>) -> Option<GitResponse> {
    match response {
        Ok(response) => response.json().unwrap_or_else(|err| {
            eprintln!("Failed to parse json response: {}", err);
            None
        }),
        Err(err) => {
            eprintln!("{}", err);
            None
        }
    }
}

fn exit_with_error(message: &str, code: i32) -> ! {
    eprintln!("{}", message);
    std::process::exit(code);
}
