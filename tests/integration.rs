use std::env;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

struct Fixture {
    root: PathBuf,
    home: PathBuf,
    global: PathBuf,
    bin: PathBuf,
    fake_gh: PathBuf,
    log: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let sequence = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before Unix epoch")
            .as_nanos();
        let root = env::temp_dir().join(format!(
            "gh-auto-switcher-test-{}-{timestamp}-{sequence}",
            std::process::id()
        ));
        fs::create_dir_all(&root).expect("create test root");
        let root = fs::canonicalize(root).expect("canonicalize test root");
        let home = root.join("home");
        let bin = root.join("bin");
        let global = root.join("global.gitconfig");
        let fake_gh = bin.join("gh");
        let log = root.join("fake-gh.log");
        fs::create_dir_all(&home).expect("create test home");
        fs::create_dir_all(&bin).expect("create test bin");
        write_executable(&fake_gh, FAKE_GH);
        Self {
            root,
            home,
            global,
            bin,
            fake_gh,
            log,
        }
    }

    fn env_command(&self, command: &str) -> Command {
        self.env_command_with_log(command, &self.log)
    }

    fn env_command_with_log(&self, command: &str, log: &Path) -> Command {
        let path = env::var_os("PATH").unwrap_or_default();
        let mut process = Command::new(command);
        process
            .env_clear()
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.root.join("xdg"))
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("GH_CONFIG_DIR", self.root.join("gh-config"))
            .env("GIT_CONFIG_GLOBAL", &self.global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("FAKE_GH_LOG", log)
            .env("GH_AUTO_SWITCHER_REAL_GH", &self.fake_gh);
        let mut paths = vec![self.bin.clone()];
        paths.extend(env::split_paths(&path));
        process.env("PATH", env::join_paths(paths).expect("join test PATH"));
        process
    }

    fn extension_command(&self, command: &Path) -> Command {
        let path = env::var_os("PATH").unwrap_or_default();
        let original_home = env::var_os("HOME").expect("test HOME");
        let cargo_home = env::var_os("CARGO_HOME").unwrap_or_else(|| {
            PathBuf::from(&original_home)
                .join(".cargo")
                .into_os_string()
        });
        let rustup_home = env::var_os("RUSTUP_HOME").unwrap_or_else(|| {
            PathBuf::from(&original_home)
                .join(".rustup")
                .into_os_string()
        });
        let mut process = Command::new(command);
        process
            .env_clear()
            .env("HOME", &self.home)
            .env("XDG_CONFIG_HOME", self.root.join("xdg"))
            .env("XDG_DATA_HOME", self.root.join("data"))
            .env("GH_CONFIG_DIR", self.root.join("gh-config"))
            .env("CARGO_HOME", cargo_home)
            .env("RUSTUP_HOME", rustup_home)
            .env("GIT_CONFIG_GLOBAL", &self.global)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("FAKE_GH_LOG", &self.log)
            .env("GH_AUTO_SWITCHER_REAL_GH", &self.fake_gh)
            .env("PATH", path);
        process
    }

    fn write_global(&self, content: &str) {
        fs::write(&self.global, content).expect("write global gitconfig");
    }

    fn run_git(&self, repo: &Path, args: &[&str]) {
        fs::create_dir_all(repo).expect("create git directory");
        let status = self
            .env_command("git")
            .args(args)
            .current_dir(repo)
            .status()
            .expect("run git");
        assert!(status.success(), "git command failed: {args:?}");
    }

    fn repo(&self, name: &str) -> PathBuf {
        let repo = self.root.join(name);
        self.run_git(&repo, &["init", "-q"]);
        repo
    }

    fn add_remote(&self, repo: &Path, url: &str) {
        self.run_git(repo, &["remote", "add", "origin", url]);
    }

    fn run_router(&self, cwd: &Path, args: &[&str]) -> Output {
        self.run_router_with_log(cwd, args, &self.log)
    }

    fn run_router_with_log(&self, cwd: &Path, args: &[&str], log: &Path) -> Output {
        let binary = binary_path();
        let mut command = self.env_command_with_log(binary.to_str().expect("binary path"), log);
        command.current_dir(cwd).args(args);
        command.output().expect("run gh-auto-switcher")
    }

    fn install_launcher_in_path(&self) {
        let target = self.bin.join("gh-auto-switcher");
        fs::copy(binary_path(), target).expect("copy launcher into test PATH");
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn binary_path() -> PathBuf {
    if let Some(path) = env::var_os("CARGO_BIN_EXE_gh-auto-switcher") {
        return PathBuf::from(path);
    }
    let test_binary = env::current_exe().expect("test executable path");
    test_binary
        .parent()
        .and_then(Path::parent)
        .expect("target directory")
        .join("gh-auto-switcher")
}

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn real_gh_path() -> PathBuf {
    let output = Command::new("sh")
        .args(["-c", "command -v gh"])
        .output()
        .expect("find real gh");
    assert!(output.status.success(), "command -v gh failed");
    PathBuf::from(
        String::from_utf8(output.stdout)
            .expect("real gh path UTF-8")
            .trim(),
    )
}

fn copy_source_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("create source copy directory");
    for entry in fs::read_dir(source).expect("read source directory") {
        let entry = entry.expect("read source entry");
        let source_path = entry.path();
        if source_path.file_name().is_some_and(|name| name == "target") {
            continue;
        }
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_source_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy source file");
        }
    }
}

fn write_executable(path: &Path, content: &str) {
    fs::write(path, content).expect("write executable");
    set_executable(path);
}

#[cfg(unix)]
fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path)
        .expect("read executable metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("set executable permissions");
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) {}

fn assert_account(fixture: &Fixture, repo: &Path, account: &str) {
    let output = fixture.run_router(repo, &["status"]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "status failed: {stdout}");
    assert!(stdout.contains(&format!("account: {account}")), "{stdout}");
}

const FAKE_GH: &str = r##"#!/bin/sh
if [ "$1" = "auth" ] && [ "$2" = "token" ]; then
    user=""
    previous=""
    for arg in "$@"; do
        if [ "$previous" = "--user" ] || [ "$previous" = "-u" ]; then
            user="$arg"
        fi
        previous="$arg"
    done
    if [ -n "${FAKE_GH_AUTH_STDOUT:-}" ]; then
        printf '%s' "$FAKE_GH_AUTH_STDOUT"
    fi
    if [ -n "${FAKE_GH_AUTH_STDERR:-}" ]; then
        printf '%s' "$FAKE_GH_AUTH_STDERR" >&2
    fi
    if [ -n "${FAKE_GH_AUTH_EXIT:-}" ]; then
        exit "$FAKE_GH_AUTH_EXIT"
    fi
    printf 'token-%s' "$user"
    exit 0
fi
{
    printf 'argc=%s\n' "$#"
    for arg in "$@"; do
        printf 'arg=<%s>\n' "$arg"
    done
    printf 'token=<%s>\n' "${GH_TOKEN-}"
    cat
} > "$FAKE_GH_LOG"
if [ -n "${FAKE_GH_READY:-}" ]; then
    : > "$FAKE_GH_READY"
fi
if [ -n "${FAKE_GH_SLEEP:-}" ]; then
    sleep "$FAKE_GH_SLEEP"
fi
exit "${FAKE_GH_EXIT:-0}"
"##;

#[test]
fn native_git_conditional_includes_select_the_account() {
    let fixture = Fixture::new();
    let work = fixture.root.join("work");
    fs::create_dir_all(&work).expect("create work directory");
    fs::write(
        fixture.root.join("work.inc"),
        "[github]\naccount = work-account\n",
    )
    .expect("write work include");
    fixture.write_global(&format!(
        "[github]\naccount = default-account\n[includeIf \"gitdir:{}/\"]\npath = work.inc\n",
        work.display()
    ));
    let repo = fixture.repo("work/repository");

    assert_account(&fixture, &repo, "work-account");
}

#[test]
fn native_gitdir_i_onbranch_and_remote_conditions_are_supported_by_git() {
    let fixture = Fixture::new();
    let case_root = fixture.root.join("CASE");
    fs::create_dir_all(&case_root).expect("create case directory");
    fs::write(
        fixture.root.join("case.inc"),
        "[github]\naccount = case-account\n",
    )
    .expect("write case include");
    fs::write(
        fixture.root.join("branch.inc"),
        "[github]\naccount = branch-account\n",
    )
    .expect("write branch include");
    fs::write(
        fixture.root.join("remote.inc"),
        "[github]\naccount = remote-account\n",
    )
    .expect("write remote include");
    fixture.write_global(&format!(
        "[github]\naccount = default-account\n[includeIf \"gitdir/i:{}/\"]\npath = case.inc\n[includeIf \"onbranch:feature/**\"]\npath = branch.inc\n[includeIf \"hasconfig:remote.*.url:https://github.com/healingpaper-solution/**\"]\npath = remote.inc\n",
        case_root.display().to_string().to_ascii_uppercase()
    ));
    let repo = fixture.repo("CASE/repository");
    assert_account(&fixture, &repo, "case-account");
    fixture.add_remote(
        &repo,
        "https://github.com/healingpaper-solution/repository.git",
    );
    fixture.run_git(&repo, &["switch", "-q", "-c", "feature/test"]);
    assert_account(&fixture, &repo, "remote-account");

    fs::write(
        fixture.root.join("nested-parent.inc"),
        "[include]\npath = nested-child.inc\n",
    )
    .expect("write nested parent");
    fs::write(
        fixture.root.join("nested-child.inc"),
        "[github]\naccount = nested-account\n",
    )
    .expect("write nested child");
    fixture.write_global(&format!(
        "[includeIf \"gitdir:{}/\"]\npath = nested-parent.inc\n",
        repo.parent().expect("repo parent").display()
    ));
    assert_account(&fixture, &repo, "nested-account");
}

#[test]
fn git_conditions_apply_to_linked_worktrees() {
    let fixture = Fixture::new();
    let repositories = fixture.root.join("repositories");
    fs::create_dir_all(&repositories).expect("create repositories directory");
    fs::write(
        fixture.root.join("worktree.inc"),
        "[github]\naccount = worktree-account\n",
    )
    .expect("write worktree include");
    fixture.write_global(&format!(
        "[includeIf \"gitdir:{}/\"]\npath = worktree.inc\n",
        repositories.display()
    ));
    let main_repo = fixture.repo("repositories/main");
    let commit = fixture
        .env_command("git")
        .args([
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "initial",
        ])
        .current_dir(&main_repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("create worktree commit");
    assert!(commit.success());
    let linked = repositories.join("linked");
    let worktree = fixture
        .env_command("git")
        .args([
            "worktree",
            "add",
            "-q",
            "-b",
            "linked",
            linked.to_str().unwrap(),
        ])
        .current_dir(&main_repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("create linked worktree");
    assert!(worktree.success());

    assert_account(&fixture, &linked, "worktree-account");
}

#[test]
fn branch_changes_are_re_evaluated_without_changing_process_directory() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("branch.inc"),
        "[github]\naccount = feature-account\n",
    )
    .expect("write branch include");
    fixture.write_global("[github]\naccount = default-account\n[includeIf \"onbranch:feature/**\"]\npath = branch.inc\n");
    let repo = fixture.repo("repository");

    assert_account(&fixture, &repo, "default-account");
    fixture.run_git(&repo, &["switch", "-q", "-c", "feature/test"]);
    assert_account(&fixture, &repo, "feature-account");
}

#[test]
fn nested_includes_and_git_config_environment_are_honored() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("parent.inc"),
        "[include]\npath = child.inc\n",
    )
    .expect("write parent include");
    fs::write(
        fixture.root.join("child.inc"),
        "[github]\naccount = nested-account\n",
    )
    .expect("write child include");
    fixture.write_global("[include]\npath = parent.inc\n");
    let repo = fixture.repo("repository");

    assert_account(&fixture, &repo, "nested-account");
}

#[test]
fn local_and_command_scoped_git_config_override_global_includes() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = global-account\n");
    let repo = fixture.repo("repository");
    fixture.run_git(
        &repo,
        &["config", "--local", "github.account", "local-account"],
    );
    assert_account(&fixture, &repo, "local-account");

    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "user"])
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "github.account")
        .env("GIT_CONFIG_VALUE_0", "command-account")
        .output()
        .expect("run command-scoped config");
    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read command scope log");
    assert!(log.contains("token=<token-command-account>"), "{log}");
}

#[test]
fn malformed_git_config_fails_without_using_a_fallback_account() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repository");
    fixture.write_global("[github\naccount = broken\n");
    let output = fixture.run_router(&repo, &["exec", "--", "api", "user"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("git config failed"), "{stderr}");
}

#[test]
fn management_commands_accept_leading_global_flags() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture.run_router(
        &repo,
        &["--hostname", "github.com", "auto-switcher", "status"],
    );

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("account: carter-hp"), "{stdout}");
    assert!(stdout.contains("target-host: github.com"), "{stdout}");
}

#[test]
fn doctor_acknowledges_explicit_token_without_an_account() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repository");
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["auto-switcher", "doctor"])
        .env("GH_TOKEN", "caller-token")
        .output()
        .expect("run doctor with explicit token");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("explicit environment token"), "{stdout}");
}

#[test]
fn help_payload_is_forwarded_to_real_gh() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture.run_router(&repo, &["exec", "--", "pr", "create", "--body", "--help"]);

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read help payload log");
    assert!(log.contains("arg=<--help>"), "{log}");
    assert!(log.contains("token=<token-carter-hp>"), "{log}");

    let output = fixture.run_router(&repo, &["exec", "--", "--help"]);
    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read help command log");
    assert!(log.contains("token=<>"), "{log}");
}

#[test]
fn remote_changes_are_re_evaluated_without_changing_process_directory() {
    let fixture = Fixture::new();
    fs::write(
        fixture.root.join("remote.inc"),
        "[github]\naccount = remote-account\n",
    )
    .expect("write remote account include");
    fixture.write_global(
        "[github]\naccount = default-account\n[includeIf \"hasconfig:remote.*.url:https://github.com/org/**\"]\npath = remote.inc\n",
    );
    let repo = fixture.repo("repository");

    assert_account(&fixture, &repo, "default-account");
    fixture.add_remote(&repo, "https://github.com/org/repository.git");
    assert_account(&fixture, &repo, "remote-account");
}

#[test]
fn wrapper_forwards_arguments_stdin_and_exit_status_without_global_mutation() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let before = fs::read_to_string(&fixture.global).expect("read global config");
    let binary = binary_path();
    let mut command = fixture.env_command(binary.to_str().expect("binary path"));
    command
        .current_dir(&repo)
        .args(["exec", "--", "pr", "list", "a b", "*.txt"])
        .env("FAKE_GH_EXIT", "23")
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env_remove("GH_ENTERPRISE_TOKEN")
        .env_remove("GITHUB_ENTERPRISE_TOKEN");
    command.stdin(std::process::Stdio::piped());
    let mut child = command.spawn().expect("spawn wrapper");
    use std::io::Write;
    child
        .stdin
        .take()
        .expect("wrapper stdin")
        .write_all(b"request-body")
        .expect("write wrapper stdin");
    let output = child.wait_with_output().expect("wait for wrapper");

    assert_eq!(output.status.code(), Some(23));
    let log = fs::read_to_string(&fixture.log).expect("read fake gh log");
    assert!(log.contains("arg=<pr>"), "{log}");
    assert!(log.contains("arg=<list>"), "{log}");
    assert!(log.contains("arg=<a b>"), "{log}");
    assert!(log.contains("arg=<*.txt>"), "{log}");
    assert!(log.contains("token=<token-carter-hp>"), "{log}");
    assert!(log.contains("request-body"), "{log}");
    assert_eq!(
        fs::read_to_string(&fixture.global).expect("read global config"),
        before
    );
}

#[test]
fn short_h_head_branch_is_not_treated_as_help() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture.run_router(&repo, &["exec", "--", "pr", "create", "-h", "feature/head"]);

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read head branch log");
    assert!(log.contains("arg=<-h>"), "{log}");
    assert!(log.contains("token=<token-carter-hp>"), "{log}");
}

#[test]
fn bare_gh_invocation_is_forwarded_with_no_arguments() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture.run_router(&repo, &["exec", "--"]);

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read bare gh log");
    assert!(log.contains("argc=0"), "{log}");
    assert!(log.contains("token=<token-carter-hp>"), "{log}");
}

#[test]
fn configured_github_account_still_routes_when_only_enterprise_token_exists() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "user"])
        .env("GH_ENTERPRISE_TOKEN", "unrelated-token")
        .output()
        .expect("run with unrelated enterprise token");

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read enterprise token log");
    assert!(log.contains("token=<token-carter-hp>"), "{log}");
}

#[test]
fn explicit_token_overrides_account_token_lookup() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "--hostname", "company.ghe.com", "user"])
        .env("GH_TOKEN", "caller-token")
        .output()
        .expect("run with explicit token");

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read fake gh log");
    assert!(log.contains("token=<caller-token>"), "{log}");
}

#[test]
fn missing_account_preserves_stock_gh_behavior() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repository");
    let output = fixture.run_router(&repo, &["exec", "--", "api", "user"]);

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read stock gh log");
    assert!(log.contains("token=<>"), "{log}");
}

#[test]
fn missing_account_ignores_conflicting_remote_hosts() {
    let fixture = Fixture::new();
    let repo = fixture.repo("repository");
    fixture.add_remote(&repo, "https://github.com/org/repository.git");
    fixture.run_git(
        &repo,
        &[
            "remote",
            "add",
            "upstream",
            "https://company.ghe.com/org/repository.git",
        ],
    );
    let output = fixture.run_router(&repo, &["exec", "--", "api", "user"]);

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read stock gh conflict log");
    assert!(log.contains("token=<>"), "{log}");
}

#[test]
fn explicit_token_bypasses_invalid_account_settings() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = \n");
    let repo = fixture.repo("repository");
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "user"])
        .env("GH_TOKEN", "caller-token")
        .output()
        .expect("run with explicit token and invalid account");

    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read explicit token log");
    assert!(log.contains("token=<caller-token>"), "{log}");
}

#[test]
fn missing_and_invalid_accounts_fail_without_fallback() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = \n");
    let repo = fixture.repo("repository");
    let output = fixture.run_router(&repo, &["exec", "--", "api", "user"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("github.account is empty"), "{stderr}");

    fixture.write_global("[github]\naccount = not/a/login\n");
    let output = fixture.run_router(&repo, &["exec", "--", "api", "user"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("invalid GitHub account"), "{stderr}");
}

#[test]
fn authentication_failure_is_actionable_and_does_not_leak_output() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "user"])
        .env("FAKE_GH_AUTH_EXIT", "7")
        .env("FAKE_GH_AUTH_STDOUT", "secret-token-output")
        .env("FAKE_GH_AUTH_STDERR", "secret-token-error")
        .output()
        .expect("run with missing auth");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("no GitHub authentication found for account"),
        "{stderr}"
    );
    assert!(!stderr.contains("token-carter-hp"), "{stderr}");
    assert!(!stderr.contains("secret-token-output"), "{stderr}");
    assert!(!stderr.contains("secret-token-error"), "{stderr}");
    assert!(
        output.stdout.is_empty(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn recognized_repository_targets_are_checked_without_scanning_payloads() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");

    let output = fixture.run_router(
        &repo,
        &[
            "exec",
            "--",
            "pr",
            "view",
            "https://company.ghe.com/org/repository/pull/1",
        ],
    );
    assert!(!output.status.success());
    assert!(!fixture.log.exists());

    let output = fixture.run_router(
        &repo,
        &[
            "exec",
            "--",
            "pr",
            "view",
            "-b",
            "https://company.ghe.com/org/repository/pull/1",
        ],
    );
    assert!(!output.status.success());
    assert!(!fixture.log.exists());

    let output = fixture.run_router(
        &repo,
        &[
            "exec",
            "--",
            "issue",
            "view",
            "-b",
            "https://company.ghe.com/org/repository/issues/1",
        ],
    );
    assert!(!output.status.success());
    assert!(!fixture.log.exists());

    let output = fixture.run_router(
        &repo,
        &[
            "exec",
            "--",
            "pr",
            "merge",
            "https://company.ghe.com/org/repository/pull/1",
        ],
    );
    assert!(!output.status.success());
    assert!(!fixture.log.exists());

    let output = fixture.run_router(
        &repo,
        &["exec", "--", "pr", "view", "feature.v2/team/change"],
    );
    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read branch target log");
    assert!(log.contains("token=<token-carter-hp>"), "{log}");

    fs::remove_file(&fixture.log).expect("clear branch target log");
    let output = fixture.run_router(
        &repo,
        &[
            "exec",
            "--",
            "repo",
            "clone",
            "company.ghe.com/org/repository",
        ],
    );
    assert!(!output.status.success());
    assert!(!fixture.log.exists());

    let output = fixture.run_router(
        &repo,
        &[
            "exec",
            "--",
            "api",
            "-Rcompany.ghe.com/org/repository",
            "user",
        ],
    );
    assert!(!output.status.success());
    assert!(!fixture.log.exists());

    let output = fixture.run_router(
        &repo,
        &[
            "exec",
            "--",
            "pr",
            "create",
            "--body-file",
            "--hostname=company.ghe.com",
        ],
    );
    assert!(output.status.success());
    let output = fixture.run_router(
        &repo,
        &["exec", "--", "pr", "create", "--body-file", "--help"],
    );
    assert!(output.status.success());
    let log = fs::read_to_string(&fixture.log).expect("read payload log");
    assert!(log.contains("arg=<--help>"), "{log}");
    assert!(log.contains("token=<token-carter-hp>"), "{log}");
}

#[cfg(unix)]
#[test]
fn launcher_rejects_non_executable_real_gh_path() {
    let fixture = Fixture::new();
    let non_executable = fixture.root.join("not-executable-gh");
    fs::write(&non_executable, "#!/bin/sh\nexit 0\n").expect("write non-executable gh");
    let repo = fixture.repo("repository");
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "user"])
        .env("GH_AUTO_SWITCHER_REAL_GH", &non_executable)
        .output()
        .expect("run with non-executable gh");

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not an executable file"));
}

#[test]
fn launcher_rejects_self_reference_before_recursing() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let binary = binary_path();
    let output = fixture
        .env_command(binary.to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "user"])
        .env("GH_AUTO_SWITCHER_REAL_GH", binary)
        .output()
        .expect("run self-reference");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("points to gh-auto-switcher itself"),
        "{stderr}"
    );
}

#[test]
fn unsupported_and_ambiguous_hosts_are_rejected_before_token_lookup() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let conflict = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "--hostname", "company.ghe.com", "user"])
        .env("GH_HOST", "github.com")
        .output()
        .expect("run conflicting hosts");
    assert!(!conflict.status.success());
    assert!(String::from_utf8_lossy(&conflict.stderr).contains("ambiguous GitHub host context"));

    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args([
            "exec",
            "--",
            "api",
            "--hostname",
            "github.example.com",
            "user",
        ])
        .output()
        .expect("run unsupported host");
    assert!(!output.status.success());
    assert!(!fixture.log.exists());

    fixture.add_remote(&repo, "https://github.com/org/repository.git");
    fixture.run_git(
        &repo,
        &[
            "remote",
            "add",
            "upstream",
            "https://git.example.com/org/repository.git",
        ],
    );
    let output = fixture.run_router(&repo, &["exec", "--", "api", "user"]);
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("ambiguous GitHub host context"), "{stderr}");
}

#[test]
fn auth_commands_bypass_account_routing() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "auth", "status"])
        .env("FAKE_GH_AUTH_EXIT", "7")
        .env("FAKE_GH_EXIT", "17")
        .output()
        .expect("run auth command");

    assert_eq!(output.status.code(), Some(17));
    let log = fs::read_to_string(&fixture.log).expect("read auth fake gh log");
    assert!(log.contains("arg=<auth>"), "{log}");
    assert!(log.contains("token=<>"), "{log}");
}

#[cfg(unix)]
#[test]
fn final_gh_process_receives_termination_signal() {
    use std::os::unix::process::ExitStatusExt;
    use std::time::Duration;

    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let ready = fixture.root.join("fake-gh-ready");
    let mut child = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(["exec", "--", "api", "user"])
        .env("FAKE_GH_READY", &ready)
        .env("FAKE_GH_SLEEP", "10")
        .spawn()
        .expect("spawn long-running gh");
    for _ in 0..40 {
        if ready.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    assert!(ready.exists(), "fake gh never reached the final process");
    child.kill().expect("terminate gh process");
    let status = child.wait().expect("wait for terminated gh");

    assert_eq!(status.signal(), Some(9));
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_reach_real_gh_unchanged() {
    use std::os::unix::ffi::OsStringExt;

    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = carter-hp\n");
    let repo = fixture.repo("repository");
    let non_utf8 = OsString::from_vec(vec![b'a', 0x80]);
    let output = fixture
        .env_command(binary_path().to_str().expect("binary path"))
        .current_dir(&repo)
        .args(vec![
            OsString::from("exec"),
            OsString::from("--"),
            OsString::from("api"),
            non_utf8,
        ])
        .output()
        .expect("run non-UTF-8 argument");

    assert!(output.status.success());
    let log = fs::read(&fixture.log).expect("read non-UTF-8 argument log");
    assert!(log.windows(8).any(|window| window == b"arg=<a\x80>"));
}

#[test]
fn actual_shell_integrations_preserve_normal_gh_usage() {
    for shell in ["bash", "zsh", "fish"] {
        let fixture = Fixture::new();
        fixture.install_launcher_in_path();
        fixture.write_global("[github]\naccount = carter-hp\n");
        let repo = fixture.repo(shell);
        let script = match shell {
            "fish" => "source (gh-auto-switcher shell-hook fish | psub); gh pr list 'a b' '*.txt'"
                .to_string(),
            _ => {
                format!("eval \"$(gh-auto-switcher shell-hook {shell})\"; gh pr list 'a b' '*.txt'")
            }
        };
        let output = fixture
            .env_command(shell)
            .current_dir(&repo)
            .args(["-c", &script])
            .output()
            .unwrap_or_else(|error| panic!("run {shell}: {error}"));
        assert!(output.status.success(), "{shell}: {:?}", output);
        let log = fs::read_to_string(&fixture.log).expect("read shell fake gh log");
        assert!(log.contains("arg=<a b>"), "{shell}: {log}");
        assert!(log.contains("arg=<*.txt>"), "{shell}: {log}");
        assert!(log.contains("token=<token-carter-hp>"), "{shell}: {log}");
    }
}

#[cfg(unix)]
#[test]
fn local_extension_install_runs_management_and_all_shell_hooks() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = work-github-username\n");
    let repo = fixture.repo("extension-consumer");
    let real_gh = real_gh_path();
    let source = project_root();

    let install = fixture
        .extension_command(&real_gh)
        .current_dir(&source)
        .args(["extension", "install", "."])
        .env("GH_TOKEN", "extension-install-token")
        .output()
        .expect("install local extension");
    assert!(install.status.success(), "{install:?}");

    let extension_link = fixture.root.join("data/gh/extensions/gh-auto-switcher");
    let metadata = fs::symlink_metadata(&extension_link).expect("read extension link");
    assert!(metadata.file_type().is_symlink());
    assert!(extension_link.starts_with(&fixture.root));
    assert_eq!(
        fs::read_link(extension_link).expect("read extension target"),
        source
    );

    let help = fixture
        .extension_command(&real_gh)
        .current_dir(&repo)
        .args(["auto-switcher", "help"])
        .output()
        .expect("run installed extension help");
    assert!(help.status.success(), "{help:?}");
    assert!(String::from_utf8_lossy(&help.stdout).contains("gh-auto-switcher"));

    for shell in ["bash", "zsh", "fish"] {
        let hook = fixture
            .extension_command(&real_gh)
            .current_dir(&repo)
            .args(["auto-switcher", "shell-hook", shell])
            .output()
            .unwrap_or_else(|error| panic!("run extension hook for {shell}: {error}"));
        assert!(hook.status.success(), "{shell}: {hook:?}");
        assert!(
            String::from_utf8_lossy(&hook.stdout).contains("command gh auto-switcher exec"),
            "{shell}: {}",
            String::from_utf8_lossy(&hook.stdout)
        );

        let script = match shell {
            "fish" => "source (gh auto-switcher shell-hook fish | psub); gh pr list 'a b' '*.txt'",
            _ => "eval \"$(gh auto-switcher shell-hook SHELL)\"; gh pr list 'a b' '*.txt'",
        }
        .replace("SHELL", shell);
        let output = fixture
            .extension_command(Path::new(shell))
            .current_dir(&repo)
            .args(["-c", &script])
            .output()
            .unwrap_or_else(|error| panic!("run extension shell for {shell}: {error}"));
        assert!(output.status.success(), "{shell}: {output:?}");
        let log = fs::read_to_string(&fixture.log).expect("read extension fake gh log");
        assert!(log.contains("arg=<a b>"), "{shell}: {log}");
        assert!(log.contains("arg=<*.txt>"), "{shell}: {log}");
        assert!(
            log.contains("token=<token-work-github-username>"),
            "{shell}: {log}"
        );
    }
}

#[cfg(unix)]
#[test]
fn prebuilt_extension_hooks_reenter_the_extension_without_standalone_path() {
    let fixture = Fixture::new();
    fixture.write_global("[github]\naccount = work-github-username\n");
    let repo = fixture.repo("prebuilt-extension-consumer");
    let real_gh = real_gh_path();
    let extension_dir = fixture.root.join("data/gh/extensions/gh-auto-switcher");
    fs::create_dir_all(&extension_dir).expect("create extension directory");
    let extension_binary = extension_dir.join("gh-auto-switcher");
    fs::copy(binary_path(), &extension_binary).expect("copy prebuilt extension");
    set_executable(&extension_binary);

    for shell in ["bash", "zsh", "fish"] {
        let hook = fixture
            .extension_command(&real_gh)
            .current_dir(&repo)
            .args(["auto-switcher", "shell-hook", shell])
            .output()
            .unwrap_or_else(|error| panic!("run prebuilt extension hook for {shell}: {error}"));
        assert!(hook.status.success(), "{shell}: {hook:?}");
        let hook_text = String::from_utf8_lossy(&hook.stdout);
        assert!(
            hook_text.contains("gh auto-switcher exec"),
            "{shell}: {hook_text}"
        );

        let script = match shell {
            "fish" => "source (gh auto-switcher shell-hook fish | psub); gh pr list 'a b' '*.txt'",
            _ => "eval \"$(gh auto-switcher shell-hook SHELL)\"; gh pr list 'a b' '*.txt'",
        }
        .replace("SHELL", shell);
        let output = fixture
            .extension_command(Path::new(shell))
            .current_dir(&repo)
            .args(["-c", &script])
            .output()
            .unwrap_or_else(|error| panic!("run prebuilt extension shell for {shell}: {error}"));
        assert!(output.status.success(), "{shell}: {output:?}");
        let log = fs::read_to_string(&fixture.log).expect("read prebuilt extension fake gh log");
        assert!(log.contains("arg=<a b>"), "{shell}: {log}");
        assert!(log.contains("arg=<*.txt>"), "{shell}: {log}");
        assert!(
            log.contains("token=<token-work-github-username>"),
            "{shell}: {log}"
        );
    }
}

#[cfg(unix)]
#[test]
fn extension_entrypoint_rebuilds_changed_source_in_a_spaced_path() {
    let fixture = Fixture::new();
    let source = fixture.root.join("extension source with spaces");
    copy_source_tree(&project_root(), &source);
    let entrypoint = source.join("gh-auto-switcher");
    set_executable(&entrypoint);
    let repo = fixture.repo("extension-rebuild-consumer");

    let first = fixture
        .extension_command(&entrypoint)
        .current_dir(&repo)
        .args(["help"])
        .output()
        .expect("run copied extension entrypoint");
    assert!(first.status.success(), "{first:?}");

    let main = source.join("src/main.rs");
    let content = fs::read_to_string(&main).expect("read copied main");
    let changed = content.replace(
        "gh-auto-switcher\\n\\nUsage:",
        "gh-auto-switcher\\n\\nsource-rebuild-sentinel\\n\\nUsage:",
    );
    assert_ne!(content, changed);
    fs::write(main, changed).expect("change copied source");

    let second = fixture
        .extension_command(&entrypoint)
        .current_dir(&repo)
        .args(["help"])
        .output()
        .expect("rebuild changed extension entrypoint");
    assert!(second.status.success(), "{second:?}");
    assert!(String::from_utf8_lossy(&second.stdout).contains("source-rebuild-sentinel"));
}

#[cfg(unix)]
#[test]
fn extension_entrypoint_reports_build_failures() {
    let fixture = Fixture::new();
    let cargo = fixture.bin.join("cargo");
    write_executable(&cargo, "#!/bin/sh\nprintf 'cargo failed\\n' >&2\nexit 23\n");
    let repo = fixture.repo("extension-build-failure-consumer");
    let source_entrypoint = project_root().join("gh-auto-switcher");
    let original_path = env::var_os("PATH").unwrap_or_default();
    let path = env::join_paths(
        [fixture.bin.clone()]
            .into_iter()
            .chain(env::split_paths(&original_path)),
    )
    .expect("join cargo test PATH");

    let output = fixture
        .extension_command(&source_entrypoint)
        .current_dir(&repo)
        .env("PATH", path)
        .args(["help"])
        .output()
        .expect("run extension with failing cargo");
    assert_eq!(output.status.code(), Some(23));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cargo failed"));
}

#[test]
fn concurrent_processes_select_accounts_without_shared_switch_state() {
    let fixture = Fixture::new();
    let first_root = fixture.root.join("first");
    let second_root = fixture.root.join("second");
    fs::create_dir_all(&first_root).expect("create first repository root");
    fs::create_dir_all(&second_root).expect("create second repository root");
    fs::write(
        fixture.root.join("first.inc"),
        "[github]\naccount = first-account\n",
    )
    .expect("write first account include");
    fs::write(
        fixture.root.join("second.inc"),
        "[github]\naccount = second-account\n",
    )
    .expect("write second account include");
    fixture.write_global(&format!(
        "[includeIf \"gitdir:{}/\"]\npath = first.inc\n[includeIf \"gitdir:{}/\"]\npath = second.inc\n",
        first_root.display(),
        second_root.display()
    ));
    let first_repo = fixture.repo("first/repository");
    let second_repo = fixture.repo("second/repository");
    let first_log = fixture.root.join("first.log");
    let second_log = fixture.root.join("second.log");

    std::thread::scope(|scope| {
        let first_thread = scope.spawn(|| {
            fixture.run_router_with_log(&first_repo, &["exec", "--", "api", "user"], &first_log)
        });
        let second_thread = scope.spawn(|| {
            fixture.run_router_with_log(&second_repo, &["exec", "--", "api", "user"], &second_log)
        });
        let first_output = first_thread.join().expect("first process");
        let second_output = second_thread.join().expect("second process");
        assert!(first_output.status.success());
        assert!(second_output.status.success());
    });

    assert!(fs::read_to_string(first_log)
        .expect("read first fake gh log")
        .contains("token=<token-first-account>"));
    assert!(fs::read_to_string(second_log)
        .expect("read second fake gh log")
        .contains("token=<token-second-account>"));
}
