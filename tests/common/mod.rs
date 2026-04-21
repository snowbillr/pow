use std::path::{Path, PathBuf};
use std::process::Command as StdCommand;

use assert_cmd::Command;
use tempfile::TempDir;

pub struct TestEnv {
    pub home: TempDir,
    pub config_path: PathBuf,
    pub workspaces_root: PathBuf,
    pub sources_root: PathBuf,
}

#[allow(dead_code)]
impl TestEnv {
    pub fn new() -> Self {
        let home = TempDir::new().unwrap();
        let config_path = home.path().join("config.toml");
        let workspaces_root = home.path().join("workspaces");
        let sources_root = home.path().join("src");
        std::fs::create_dir_all(&workspaces_root).unwrap();
        std::fs::create_dir_all(&sources_root).unwrap();
        TestEnv {
            home,
            config_path,
            workspaces_root,
            sources_root,
        }
    }

    pub fn pow(&self) -> Command {
        let mut cmd = Command::cargo_bin("pow").unwrap();
        cmd.env_clear()
            .env("HOME", self.home.path())
            .env("PATH", std::env::var_os("PATH").unwrap_or_default())
            .env("POW_CONFIG", &self.config_path)
            .env("POW_WORKSPACES_ROOT", &self.workspaces_root);
        cmd
    }

    pub fn make_fake_repo(&self, path: &Path) {
        std::fs::create_dir_all(path).unwrap();
        run_git(path, &["init", "-q", "-b", "main"]);
        run_git(
            path,
            &[
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "commit",
                "--allow-empty",
                "-q",
                "-m",
                "init",
            ],
        );
    }

    pub fn make_source(&self, source_name: &str, repos: &[&str]) -> PathBuf {
        let dir = self.sources_root.join(source_name);
        std::fs::create_dir_all(&dir).unwrap();
        for r in repos {
            self.make_fake_repo(&dir.join(r));
        }
        dir
    }
}

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
