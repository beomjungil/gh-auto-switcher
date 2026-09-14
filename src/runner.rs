use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::{auth, gitconfig, target};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialMode {
    Stock,
    ExplicitEnvironment,
    ProcessLocalAccount(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoutingResolution {
    pub account: Option<gitconfig::AccountSelection>,
    pub host: Option<target::HostResolution>,
    pub credential_mode: CredentialMode,
}

pub fn resolve_routing(cwd: &Path, args: &[OsString]) -> Result<RoutingResolution, String> {
    let account = gitconfig::resolve_account(cwd);
    let must_resolve_host = account.as_ref().map_or(true, |account| {
        !matches!(account, gitconfig::AccountSelection::Stock)
    }) || has_any_caller_token();

    if !must_resolve_host {
        return Ok(RoutingResolution {
            account: Some(account.expect("stock account resolution")),
            host: None,
            credential_mode: CredentialMode::Stock,
        });
    }

    let host = target::resolve_host(cwd, args)?;
    if has_explicit_token_for_host(&host.host) {
        return Ok(RoutingResolution {
            account: None,
            host: Some(host),
            credential_mode: CredentialMode::ExplicitEnvironment,
        });
    }

    let account = account?;
    let credential_mode = match &account {
        gitconfig::AccountSelection::Stock => CredentialMode::Stock,
        gitconfig::AccountSelection::Configured(account) => {
            CredentialMode::ProcessLocalAccount(account.username.clone())
        }
        gitconfig::AccountSelection::Override(account) => {
            CredentialMode::ProcessLocalAccount(account.clone())
        }
    };

    if !target::is_supported(&host.host) && !matches!(account, gitconfig::AccountSelection::Stock) {
        return Err(format!(
            "automatic routing only supports github.com; found {:?} from {}. Set an applicable token explicitly or run the real gh directly",
            host.host, host.source
        ));
    }

    Ok(RoutingResolution {
        account: Some(account),
        host: Some(host),
        credential_mode,
    })
}

pub fn execute(args: &[OsString]) -> Result<(), String> {
    let real_gh = find_real_gh()?;

    if is_auth_command(args) || is_no_auth_command(args) {
        return exec_real_gh(&real_gh, args, None);
    }

    let routing = resolve_routing(Path::new("."), args)?;
    match routing.credential_mode {
        CredentialMode::ExplicitEnvironment | CredentialMode::Stock => {
            exec_real_gh(&real_gh, args, None)
        }
        CredentialMode::ProcessLocalAccount(account) => {
            let token = auth::token_for(&real_gh, &account)?;
            exec_real_gh(&real_gh, args, Some(&token))
        }
    }
}

pub fn find_real_gh() -> Result<PathBuf, String> {
    let current_exe = env::current_exe()
        .map_err(|error| format!("failed to resolve gh-auto-switcher path: {error}"))?;
    let current_exe = canonical_or_original(&current_exe);

    if let Some(configured) = env::var_os("GH_AUTO_SWITCHER_REAL_GH") {
        let configured = PathBuf::from(configured);
        let resolved = canonical_or_original(&configured);
        if same_path(&resolved, &current_exe) {
            return Err("GH_AUTO_SWITCHER_REAL_GH points to gh-auto-switcher itself".to_string());
        }
        if !is_executable_file(&configured) {
            return Err(format!(
                "GH_AUTO_SWITCHER_REAL_GH is not an executable file: {}",
                configured.display()
            ));
        }
        return Ok(configured);
    }

    let path = env::var_os("PATH").ok_or_else(|| "PATH is not set".to_string())?;
    for directory in env::split_paths(&path) {
        let candidate = directory.join("gh");
        if !is_executable_file(&candidate) {
            continue;
        }
        let resolved = canonical_or_original(&candidate);
        if !same_path(&resolved, &current_exe) {
            return Ok(candidate);
        }
    }

    Err("could not find the real gh executable; set GH_AUTO_SWITCHER_REAL_GH".to_string())
}

fn exec_real_gh(real_gh: &Path, args: &[OsString], token: Option<&str>) -> Result<(), String> {
    let mut command = Command::new(real_gh);
    command.args(args);
    if let Some(token) = token {
        command.env("GH_TOKEN", token);
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        let error = command.exec();
        Err(format!("failed to execute real gh: {error}"))
    }

    #[cfg(not(unix))]
    {
        let status = command
            .status()
            .map_err(|error| format!("failed to execute real gh: {error}"))?;
        std::process::exit(status.code().unwrap_or(1));
    }
}

pub fn is_management_command(args: &[OsString]) -> bool {
    first_command(args) == Some(OsStr::new("auto-switcher"))
}

fn is_auth_command(args: &[OsString]) -> bool {
    first_command(args) == Some(OsStr::new("auth"))
}

fn is_no_auth_command(args: &[OsString]) -> bool {
    let Some(command_index) = first_command_index(args) else {
        return args.len() == 1 && args[0] == OsStr::new("--help");
    };
    let command = &args[command_index];
    command == OsStr::new("help")
        || command == OsStr::new("version")
        || command == OsStr::new("completion")
        || command == OsStr::new("config")
        || command == OsStr::new("alias")
}

fn first_command(args: &[OsString]) -> Option<&OsStr> {
    first_command_index(args).and_then(|index| args.get(index).map(OsString::as_os_str))
}

pub fn first_command_index(args: &[OsString]) -> Option<usize> {
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == OsStr::new("--") {
            return None;
        }
        if arg == OsStr::new("--hostname") || arg == OsStr::new("--repo") || arg == OsStr::new("-R")
        {
            index += 2;
            continue;
        }
        if arg.to_str().is_some_and(|arg| {
            arg.starts_with("--hostname=")
                || arg.starts_with("--repo=")
                || arg.starts_with("-R")
                || arg.starts_with('-')
        }) {
            index += 1;
            continue;
        }
        return Some(index);
    }
    None
}

fn has_any_caller_token() -> bool {
    [
        "GH_TOKEN",
        "GITHUB_TOKEN",
        "GH_ENTERPRISE_TOKEN",
        "GITHUB_ENTERPRISE_TOKEN",
    ]
    .iter()
    .any(|name| env::var_os(name).is_some_and(|value| !value.is_empty()))
}

fn has_explicit_token_for_host(host: &str) -> bool {
    let names = if target::is_supported(host) || host.ends_with(".ghe.com") {
        ["GH_TOKEN", "GITHUB_TOKEN"]
    } else {
        ["GH_ENTERPRISE_TOKEN", "GITHUB_ENTERPRISE_TOKEN"]
    };
    names
        .iter()
        .any(|name| env::var_os(name).is_some_and(|value| !value.is_empty()))
}

fn is_executable_file(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::metadata(path)
            .map(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        std::fs::metadata(path)
            .map(|metadata| metadata.is_file())
            .unwrap_or(false)
    }
}

fn canonical_or_original(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}
