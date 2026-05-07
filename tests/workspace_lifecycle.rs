mod common;

use common::TestEnv;

#[test]
fn full_lifecycle() {
    let env = TestEnv::new();
    let source_dir = env.make_source("Babylist", &["family-ties", "StudioOne", "web"]);

    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&source_dir)
        .assert()
        .success();

    env.pow().args(["new", "studio"]).assert().success();

    env.pow()
        .args(["add", "family-ties"])
        .env("POW_ACTIVE", "studio")
        .assert()
        .success();

    env.pow()
        .args(["add", "StudioOne"])
        .env("POW_ACTIVE", "studio")
        .assert()
        .success();

    // Both worktrees exist at the right paths.
    let ws = env.workspaces_root.join("studio");
    assert!(ws.join("family-ties").is_dir());
    assert!(ws.join("StudioOne").is_dir());

    // Branch 'studio' exists in each source repo.
    for repo in ["family-ties", "StudioOne"] {
        let out = std::process::Command::new("git")
            .args([
                "-C",
                &source_dir.join(repo).to_string_lossy(),
                "branch",
                "--list",
                "studio",
            ])
            .output()
            .unwrap();
        let s = String::from_utf8_lossy(&out.stdout);
        assert!(s.contains("studio"), "branch missing in {repo}: {s}");
    }

    // forget removes one
    env.pow()
        .args(["forget", "family-ties"])
        .env("POW_ACTIVE", "studio")
        .assert()
        .success();
    assert!(!ws.join("family-ties").is_dir());
    assert!(ws.join("StudioOne").is_dir());

    // rm without --prune-branches: tears down but leaves branches
    env.pow()
        .args(["rm", "studio", "--force"])
        .assert()
        .success();
    assert!(!ws.exists());

    let out = std::process::Command::new("git")
        .args([
            "-C",
            &source_dir.join("StudioOne").to_string_lossy(),
            "branch",
            "--list",
            "studio",
        ])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(s.contains("studio"), "branch should still exist: {s}");
}

#[test]
fn rm_with_prune_branches_deletes_branch() {
    let env = TestEnv::new();
    let source_dir = env.make_source("Babylist", &["web"]);

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source_dir)
        .assert()
        .success();
    env.pow().args(["new", "ws1"]).assert().success();
    env.pow()
        .args(["add", "web"])
        .env("POW_ACTIVE", "ws1")
        .assert()
        .success();

    env.pow()
        .args(["rm", "ws1", "--force", "--prune-branches"])
        .assert()
        .success();

    let out = std::process::Command::new("git")
        .args([
            "-C",
            &source_dir.join("web").to_string_lossy(),
            "branch",
            "--list",
            "ws1",
        ])
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&out.stdout);
    assert!(!s.contains("ws1"), "branch should be gone: {s}");
}

#[test]
fn ambiguous_repo_name_errors() {
    let env = TestEnv::new();
    let a = env.make_source("A", &["common"]);
    let b = env.make_source("B", &["common"]);

    env.pow()
        .args(["source", "add", "a"])
        .arg(&a)
        .assert()
        .success();
    env.pow()
        .args(["source", "add", "b"])
        .arg(&b)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();

    let output = env
        .pow()
        .args(["add", "common"])
        .env("POW_ACTIVE", "ws")
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ambiguous"), "stderr: {stderr}");
    assert_eq!(output.status.code(), Some(3));
}

#[test]
fn qualified_repo_name_resolves() {
    let env = TestEnv::new();
    let a = env.make_source("A", &["common"]);
    let b = env.make_source("B", &["common"]);

    env.pow()
        .args(["source", "add", "a"])
        .arg(&a)
        .assert()
        .success();
    env.pow()
        .args(["source", "add", "b"])
        .arg(&b)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();

    env.pow()
        .args(["add", "a/common"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    assert!(env.workspaces_root.join("ws").join("common").is_dir());
}

#[test]
fn add_multiple_repos_in_one_invocation() {
    let env = TestEnv::new();
    let source_dir = env.make_source("Babylist", &["family-ties", "StudioOne", "web"]);

    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&source_dir)
        .assert()
        .success();
    env.pow().args(["new", "studio"]).assert().success();

    env.pow()
        .args(["add", "family-ties", "StudioOne", "web"])
        .env("POW_ACTIVE", "studio")
        .assert()
        .success();

    let ws = env.workspaces_root.join("studio");
    assert!(ws.join("family-ties").is_dir());
    assert!(ws.join("StudioOne").is_dir());
    assert!(ws.join("web").is_dir());
}

#[test]
fn add_multiple_repos_partial_failure_continues() {
    let env = TestEnv::new();
    let source_dir = env.make_source("Babylist", &["family-ties", "web"]);

    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&source_dir)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();

    let output = env
        .pow()
        .args(["add", "family-ties", "does-not-exist", "web"])
        .env("POW_ACTIVE", "ws")
        .output()
        .unwrap();
    assert!(!output.status.success(), "expected non-zero exit");

    let ws = env.workspaces_root.join("ws");
    assert!(ws.join("family-ties").is_dir(), "first repo should be added");
    assert!(ws.join("web").is_dir(), "third repo should be added");
    assert!(!ws.join("does-not-exist").exists());
}

#[test]
fn add_without_workspace_or_active_errors() {
    let env = TestEnv::new();
    let dir = env.make_source("bl", &["repo"]);
    env.pow()
        .args(["source", "add", "bl"])
        .arg(&dir)
        .assert()
        .success();

    let output = env.pow().args(["add", "repo"]).output().unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("POW_ACTIVE"), "stderr: {stderr}");
}
