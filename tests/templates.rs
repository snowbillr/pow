mod common;

use std::fs;
use std::io::Write;

use common::TestEnv;
use predicates::prelude::*;

fn append_template(env: &TestEnv, name: &str, repos: &[&str]) {
    let mut f = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(&env.config_path)
        .unwrap();
    writeln!(f, "\n[[templates]]").unwrap();
    writeln!(f, "name = \"{name}\"").unwrap();
    let joined = repos
        .iter()
        .map(|r| format!("\"{r}\""))
        .collect::<Vec<_>>()
        .join(", ");
    writeln!(f, "repos = [{joined}]").unwrap();
}

#[test]
fn template_list_shows_configured_templates() {
    let env = TestEnv::new();
    let dir = env.make_source("Babylist", &["web", "api"]);
    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&dir)
        .assert()
        .success();

    append_template(&env, "frontend", &["babylist/web", "babylist/api"]);

    env.pow()
        .args(["template", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("frontend"))
        .stdout(predicate::str::contains("babylist/web"));

    env.pow()
        .args(["template", "list", "--json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"frontend\""));
}

#[test]
fn template_list_when_empty() {
    let env = TestEnv::new();
    env.pow()
        .args(["template", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("no templates configured"));
}

#[test]
fn new_with_template_adds_all_repos() {
    let env = TestEnv::new();
    let dir = env.make_source("Babylist", &["web", "api"]);
    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&dir)
        .assert()
        .success();
    append_template(&env, "frontend", &["babylist/web", "babylist/api"]);

    env.pow()
        .args(["new", "studio", "-t", "frontend"])
        .assert()
        .success();

    let ws = env.workspaces_root.join("studio");
    assert!(ws.join("web").is_dir());
    assert!(ws.join("api").is_dir());

    // Both branches named after the workspace.
    for repo in ["web", "api"] {
        let out = std::process::Command::new("git")
            .args([
                "-C",
                &dir.join(repo).to_string_lossy(),
                "branch",
                "--list",
                "studio",
            ])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&out.stdout);
        assert!(s.contains("studio"), "branch missing in {repo}: {s}");
    }
}

#[test]
fn new_with_template_propagates_from() {
    let env = TestEnv::new();
    let dir = env.make_source("Babylist", &["web"]);
    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&dir)
        .assert()
        .success();

    // Create an alternate base branch in the source repo, then point HEAD on
    // it at a new commit so we can detect that the workspace branch was
    // forked from it (not from main).
    let repo_path = dir.join("web");
    let run = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .current_dir(&repo_path)
            .output()
            .unwrap()
    };
    run(&["checkout", "-q", "-b", "develop"]);
    run(&[
        "-c",
        "user.email=t@t",
        "-c",
        "user.name=t",
        "commit",
        "--allow-empty",
        "-q",
        "-m",
        "develop-only",
    ]);
    run(&["checkout", "-q", "main"]);

    let develop_sha = String::from_utf8(run(&["rev-parse", "develop"]).stdout)
        .unwrap()
        .trim()
        .to_string();

    append_template(&env, "frontend", &["babylist/web"]);

    env.pow()
        .args(["new", "studio", "-t", "frontend", "-f", "develop"])
        .assert()
        .success();

    let merge_base = String::from_utf8(run(&["merge-base", "studio", "develop"]).stdout)
        .unwrap()
        .trim()
        .to_string();
    assert_eq!(
        merge_base, develop_sha,
        "studio should be forked from develop"
    );
}

#[test]
fn new_with_template_continues_on_failure() {
    let env = TestEnv::new();
    let dir = env.make_source("Babylist", &["web"]);
    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&dir)
        .assert()
        .success();

    append_template(&env, "frontend", &["babylist/web", "babylist/missing"]);

    let out = env
        .pow()
        .args(["new", "studio", "-t", "frontend"])
        .output()
        .unwrap();
    assert!(!out.status.success(), "expected non-zero exit");

    // The valid one was added.
    assert!(env.workspaces_root.join("studio").join("web").is_dir());

    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Failed:"), "stderr: {stderr}");
    assert!(stderr.contains("babylist/missing"), "stderr: {stderr}");
}

#[test]
fn new_with_unknown_template_errors() {
    let env = TestEnv::new();
    let out = env
        .pow()
        .args(["new", "studio", "-t", "nope"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("template 'nope' not found"), "{stderr}");
    // Workspace dir should not have been created.
    assert!(!env.workspaces_root.join("studio").exists());
}
