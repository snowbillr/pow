mod common;

use std::path::Path;
use std::process::Command as StdCommand;

use common::TestEnv;

fn run_git(cwd: &Path, args: &[&str]) {
    let out = StdCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .expect("failed to run git");
    assert!(
        out.status.success(),
        "git {args:?} in {}: {}",
        cwd.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn commit_pow_toml(repo: &Path, body: &str) {
    std::fs::write(repo.join(".pow.toml"), body).unwrap();
    run_git(repo, &["add", ".pow.toml"]);
    run_git(
        repo,
        &[
            "-c",
            "user.email=t@t",
            "-c",
            "user.name=t",
            "commit",
            "-q",
            "-m",
            "add pow.toml",
        ],
    );
}

#[test]
fn add_runs_setup_commands() {
    let env = TestEnv::new();
    let source = env.make_source("bl", &["foo"]);
    commit_pow_toml(
        &source.join("foo"),
        "[setup]\ncommands = [\"touch SETUP_RAN\"]\n",
    );

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();
    env.pow()
        .args(["add", "foo"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    assert!(env.workspaces_root.join("ws/foo/SETUP_RAN").exists());
}

#[test]
fn add_copies_files() {
    let env = TestEnv::new();
    let source = env.make_source("bl", &["foo"]);
    commit_pow_toml(&source.join("foo"), "[setup]\ncopy = [\".env\"]\n");
    std::fs::write(source.join("foo/.env"), "FOO=bar\n").unwrap();

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();
    env.pow()
        .args(["add", "foo"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    let copied = env.workspaces_root.join("ws/foo/.env");
    assert!(copied.exists(), "copied file missing");
    assert_eq!(std::fs::read_to_string(copied).unwrap(), "FOO=bar\n");
}

#[test]
fn add_no_setup_skips_everything() {
    let env = TestEnv::new();
    let source = env.make_source("bl", &["foo"]);
    commit_pow_toml(
        &source.join("foo"),
        "[setup]\ncommands = [\"touch SETUP_RAN\"]\ncopy = [\".env\"]\n",
    );
    std::fs::write(source.join("foo/.env"), "x").unwrap();

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();
    env.pow()
        .args(["add", "foo", "--no-setup"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    assert!(!env.workspaces_root.join("ws/foo/SETUP_RAN").exists());
    assert!(!env.workspaces_root.join("ws/foo/.env").exists());
}

#[test]
fn add_succeeds_when_command_fails() {
    let env = TestEnv::new();
    let source = env.make_source("bl", &["foo"]);
    commit_pow_toml(&source.join("foo"), "[setup]\ncommands = [\"false\"]\n");

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();
    env.pow()
        .args(["add", "foo"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    assert!(env.workspaces_root.join("ws/foo").is_dir());
}

#[test]
fn add_with_no_pow_toml_is_noop() {
    let env = TestEnv::new();
    let source = env.make_source("bl", &["foo"]);

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();
    env.pow()
        .args(["add", "foo"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    assert!(env.workspaces_root.join("ws/foo").is_dir());
}

#[test]
fn sync_recopies_files() {
    let env = TestEnv::new();
    let source = env.make_source("bl", &["foo"]);
    commit_pow_toml(&source.join("foo"), "[setup]\ncopy = [\".env\"]\n");
    std::fs::write(source.join("foo/.env"), "FOO=bar\n").unwrap();

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();
    env.pow()
        .args(["add", "foo"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    // Mutate the source's .env after the initial copy.
    std::fs::write(source.join("foo/.env"), "FOO=baz\n").unwrap();

    env.pow()
        .args(["sync"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    let copied = std::fs::read_to_string(env.workspaces_root.join("ws/foo/.env")).unwrap();
    assert_eq!(copied, "FOO=baz\n", "sync should re-copy .env");
}

#[test]
fn sync_does_not_rerun_commands() {
    let env = TestEnv::new();
    let source = env.make_source("bl", &["foo"]);
    commit_pow_toml(
        &source.join("foo"),
        "[setup]\ncommands = [\"touch RAN\"]\ncopy = [\".env\"]\n",
    );
    std::fs::write(source.join("foo/.env"), "x\n").unwrap();

    env.pow()
        .args(["source", "add", "bl"])
        .arg(&source)
        .assert()
        .success();
    env.pow().args(["new", "ws"]).assert().success();
    env.pow()
        .args(["add", "foo"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    let ran = env.workspaces_root.join("ws/foo/RAN");
    assert!(ran.exists(), "command should run on add");
    std::fs::remove_file(&ran).unwrap();

    env.pow()
        .args(["sync"])
        .env("POW_ACTIVE", "ws")
        .assert()
        .success();

    assert!(!ran.exists(), "sync must not rerun setup commands");
}
