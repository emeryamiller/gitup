use std::process::{exit, Command, Output};
use std::str::from_utf8;
use url::Url;

const GIT_COMMAND_FAILED: i32 = 10;
const GIT_RETURNED_NOTHING: i32 = 11;

pub fn current_branch() -> String {
    let result = Command::new("git")
        .arg("branch")
        .arg("--show-current")
        .output();

    let stdout = handle_git_output(result);
    utf8_from_bytes(&stdout, "git branch")
}

pub fn is_remote_branch() -> bool {
    let result = Command::new("git").arg("status").arg("-sb").output();

    let stdout = handle_git_output(result);
    let line = match stdout.split(|&c| c == b'\n').next() {
        Some(first_line) => utf8_from_bytes(&first_line, "git status"),
        None => print_error_and_exit("Git status returned nothing", GIT_RETURNED_NOTHING),
    };

    line.starts_with("## ") && line.contains("...")
}

pub fn git_add() {
    let result = Command::new("git").arg("add").arg(".").output();
    handle_git_output(result);
}

pub fn git_commit(message: &str) {
    let result = Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg(message)
        .output();
    handle_git_output(result);
}

pub fn git_amend() {
    let result = Command::new("git")
        .arg("commit")
        .arg("--amend")
        .arg("--no-edit")
        .output();
    handle_git_output(result);
}

pub fn git_push(force: bool) -> String {
    let mut command: Command = Command::new("git");
    command.arg("push");
    if force {
        command.arg("--force-with-lease");
    }
    let result = command.output();
    let stdout = handle_git_output(result);

    utf8_from_bytes(&stdout, "git push")
}

pub fn basic_commit(message: &str) -> Option<String> {
    println!("Committing...");
    git_add();
    git_commit(message);

    let output = git_push(false);
    println!("{}", &output);
    output
        .lines()
        .find(|line| line.starts_with("https") && line.contains("/pull/"))
        .map(|line| line.trim().to_string())
}

pub fn amend_commit() {
    println!("Amending commit");
    git_add();
    git_amend();
    git_push(true);
}

pub fn repo_name() -> Option<String> {
    let result = Command::new("git")
        .arg("remote")
        .arg("get-url")
        .arg("origin")
        .output();
    let stdout = handle_git_output(result);
    let url_string = from_utf8(&stdout)
        .expect("git remote not a UTF-8 string")
        .trim()
        .replace(".git", "");

    if url_string.is_empty() {
        None
    } else {
        let url = Url::parse(&url_string).expect(
            format!(
                "git remote url `{}` sting failed to parse to a url",
                url_string
            )
            .as_str(),
        );
        let mut path = url.path().to_string();
        path.remove(0);
        Some(path)
    }
}

pub fn remote_branch_name() -> Option<String> {
    Command::new("git")
        .arg("rev-parse")
        .arg("--abbrev-ref")
        .arg("--symbolic-full-name")
        .arg("@{u}")
        .output()
        .map(|output| Some(utf8_from_bytes(&output.stdout, "git rev-parse")))
        .unwrap_or(None)
}

pub fn remote_commit_id(branch: &str) -> Option<String> {
    Command::new("git")
        .arg("log")
        .arg("-n")
        .arg("1")
        .arg("--pretty=format:%H")
        .arg(branch)
        .output()
        .map(|output| Some(utf8_from_bytes(&output.stdout, "git log")))
        .unwrap_or(None)
}

fn handle_git_output(result: Result<Output, std::io::Error>) -> Vec<u8> {
    match result {
        Ok(output) => {
            if output.status.success() {
                output.stdout
            } else {
                print_error_and_exit(
                    &format!(
                        "Git command failed with error code {}: {}",
                        &output.status.code().unwrap_or(-1),
                        utf8_from_bytes(&output.stderr, "git command stderr")
                    ),
                    GIT_COMMAND_FAILED,
                );
            }
        }
        Err(error) => {
            print_error_and_exit(&format!("Git command error: {}", error), GIT_COMMAND_FAILED);
        }
    }
}

fn print_error_and_exit(message: &str, error_code: i32) -> ! {
    eprintln!("{}", message);
    exit(error_code);
}

fn utf8_from_bytes(output: &[u8], desc: &str) -> String {
    from_utf8(output)
        .expect(&format!("Could not parse {} to UTF-8 string", desc))
        .trim()
        .to_string()
}
