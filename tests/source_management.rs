mod common;

use common::TestEnv;
use predicates::prelude::*;

#[test]
fn add_list_remove() {
    let env = TestEnv::new();
    let dir = env.make_source("Babylist", &["a", "b"]);

    env.pow()
        .args(["source", "add", "babylist"])
        .arg(&dir)
        .assert()
        .success();
    env.pow()
        .args(["source", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("babylist"));
    env.pow()
        .args(["source", "remove", "babylist", "--force"])
        .assert()
        .success();
}

#[test]
fn duplicate_name_errors() {
    let env = TestEnv::new();
    let dir = env.make_source("Babylist", &["a"]);

    env.pow()
        .args(["source", "add", "x"])
        .arg(&dir)
        .assert()
        .success();
    let out = env
        .pow()
        .args(["source", "add", "x"])
        .arg(&dir)
        .output()
        .unwrap();
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("already exists"), "{stderr}");
}

#[test]
fn remove_unknown_errors() {
    let env = TestEnv::new();
    let out = env
        .pow()
        .args(["source", "remove", "nope", "--force"])
        .output()
        .unwrap();
    assert!(!out.status.success());
    assert_eq!(out.status.code(), Some(4));
}
