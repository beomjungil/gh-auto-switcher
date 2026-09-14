use std::ffi::{OsStr, OsString};
use std::path::Path;

use crate::gitconfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostResolution {
    pub host: String,
    pub source: String,
}

pub fn resolve_host(cwd: &Path, args: &[OsString]) -> Result<HostResolution, String> {
    let mut candidates = Vec::new();
    candidates.extend(hostname_flags(args)?);

    if let Some(host) = non_empty_env("GH_HOST")? {
        candidates.push((host, "GH_HOST".to_string()));
    }
    if let Some(repo) = non_empty_env("GH_REPO")? {
        if let Some(host) = repository_reference_host(&repo) {
            candidates.push((host, "GH_REPO".to_string()));
        }
    }

    candidates.extend(repository_flag_hosts(args)?);
    candidates.extend(repository_target_url_hosts(args));

    for remote in gitconfig::remote_urls(cwd)? {
        if let Some(host) = repository_url_host(&remote) {
            candidates.push((host, "Git remote".to_string()));
        }
    }

    let Some((first_host, first_source)) = candidates.first() else {
        return Ok(HostResolution {
            host: "github.com".to_string(),
            source: "default".to_string(),
        });
    };
    let first_host = normalize_host(first_host);

    if candidates
        .iter()
        .any(|(host, _)| normalize_host(host) != first_host)
    {
        let hosts = candidates
            .iter()
            .map(|(host, _)| normalize_host(host))
            .collect::<Vec<_>>()
            .join(", ");
        return Err(format!(
            "ambiguous GitHub host context ({hosts}); use one explicit host or run the real gh directly"
        ));
    }

    Ok(HostResolution {
        host: first_host,
        source: first_source.clone(),
    })
}

pub fn is_supported(host: &str) -> bool {
    normalize_host(host) == "github.com"
}

pub fn normalize_host(host: &str) -> String {
    host.trim().trim_end_matches('.').to_ascii_lowercase()
}

fn hostname_flags(args: &[OsString]) -> Result<Vec<(String, String)>, String> {
    let (command, subcommand) = command_context(args);
    let mut candidates = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == OsStr::new("--") {
            break;
        }
        if let Some(host) = arg.to_str().and_then(|arg| arg.strip_prefix("--hostname=")) {
            if host.is_empty() {
                return Err("--hostname cannot be empty".to_string());
            }
            candidates.push((host.to_string(), "--hostname".to_string()));
            index += 1;
            continue;
        }
        if arg == OsStr::new("--hostname") {
            let host = args
                .get(index + 1)
                .ok_or_else(|| "--hostname requires a value".to_string())?
                .to_str()
                .ok_or_else(|| "--hostname must be valid UTF-8".to_string())?;
            if host.is_empty() {
                return Err("--hostname cannot be empty".to_string());
            }
            candidates.push((host.to_string(), "--hostname".to_string()));
            index += 2;
            continue;
        }
        if arg
            .to_str()
            .is_some_and(|arg| option_takes_value(command, subcommand, arg))
        {
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(candidates)
}

fn repository_flag_hosts(args: &[OsString]) -> Result<Vec<(String, String)>, String> {
    let (command, subcommand) = command_context(args);
    let mut hosts = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == OsStr::new("--") {
            break;
        }

        let (value, source, consumed) = if let Some(arg) = arg.to_str() {
            if let Some(value) = arg.strip_prefix("--repo=") {
                (Some(value.to_string()), "--repo".to_string(), 1)
            } else if let Some(value) = arg.strip_prefix("-R=") {
                (Some(value.to_string()), "-R".to_string(), 1)
            } else if let Some(value) = arg.strip_prefix("-R").filter(|value| !value.is_empty()) {
                (Some(value.to_string()), "-R".to_string(), 1)
            } else if arg == "--repo" || arg == "-R" {
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("{arg} requires a repository"))?
                    .to_str()
                    .ok_or_else(|| format!("{arg} repository must be valid UTF-8"))?;
                (Some(value.to_string()), arg.to_string(), 2)
            } else if option_takes_value(command, subcommand, arg) {
                (None, String::new(), 2)
            } else {
                (None, String::new(), 1)
            }
        } else {
            (None, String::new(), 1)
        };

        if let Some(value) = value {
            if let Some(host) = repository_reference_host(&value) {
                hosts.push((host, source));
            }
        }
        index += consumed;
    }
    Ok(hosts)
}

fn repository_target_url_hosts(args: &[OsString]) -> Vec<(String, String)> {
    let Some(command_index) = first_command_index(args) else {
        return Vec::new();
    };
    let command = args.get(command_index).and_then(|arg| arg.to_str());
    let subcommand = args.get(command_index + 1).and_then(|arg| arg.to_str());
    let recognized = matches!(
        (command, subcommand),
        (Some("repo"), Some("clone" | "view"))
            | (Some("pr"), Some("merge" | "view"))
            | (Some("issue"), Some("view"))
            | (Some("discussion"), Some("view"))
    );
    if !recognized {
        return Vec::new();
    }

    let mut index = command_index + 2;
    while index < args.len() {
        let Some(arg) = args[index].to_str() else {
            index += 1;
            continue;
        };
        if arg == "--" {
            index += 1;
            break;
        }
        if arg.starts_with('-') {
            if option_takes_value(command, subcommand, arg) {
                index += 2;
            } else {
                index += 1;
            }
            continue;
        }
        return repository_target_host(command, subcommand, arg)
            .map(|host| vec![(host, "repository target URL".to_string())])
            .unwrap_or_default();
    }

    args.get(index)
        .and_then(|arg| arg.to_str())
        .and_then(|arg| repository_target_host(command, subcommand, arg))
        .map(|host| vec![(host, "repository target URL".to_string())])
        .unwrap_or_default()
}

pub fn first_command_index(args: &[OsString]) -> Option<usize> {
    let mut index = 0;
    while index < args.len() {
        match args[index].to_str() {
            Some("--") => return None,
            Some("--hostname" | "--repo" | "-R") => index += 2,
            Some(argument) if argument.starts_with("--hostname=") => index += 1,
            Some(argument) if argument.starts_with("--repo=") => index += 1,
            Some(argument) if argument.starts_with("-R") => index += 1,
            Some(argument) if argument.starts_with('-') => index += 1,
            Some(_) => return Some(index),
            None => return None,
        }
    }
    None
}

fn command_context(args: &[OsString]) -> (Option<&str>, Option<&str>) {
    let Some(command_index) = first_command_index(args) else {
        return (None, None);
    };
    (
        args.get(command_index).and_then(|arg| arg.to_str()),
        args.get(command_index + 1).and_then(|arg| arg.to_str()),
    )
}

fn option_takes_value(command: Option<&str>, subcommand: Option<&str>, arg: &str) -> bool {
    if matches!(command, Some("pr" | "issue")) && subcommand == Some("view") && arg == "-b" {
        return false;
    }
    matches!(
        arg,
        "--repo"
            | "-R"
            | "--hostname"
            | "--json"
            | "--jq"
            | "--template"
            | "--field"
            | "--limit"
            | "--branch"
            | "--base"
            | "--head"
            | "--label"
            | "--assignee"
            | "--milestone"
            | "--reviewer"
            | "--project"
            | "--comment"
            | "--body"
            | "--body-file"
            | "--title"
            | "-b"
            | "-B"
            | "-H"
            | "-t"
            | "-F"
            | "-f"
    )
}

fn repository_target_host(
    command: Option<&str>,
    subcommand: Option<&str>,
    value: &str,
) -> Option<String> {
    if command == Some("repo") && matches!(subcommand, Some("clone" | "view")) {
        repository_reference_host(value)
    } else {
        repository_url_host(value)
    }
}

fn repository_reference_host(value: &str) -> Option<String> {
    repository_url_host(value).or_else(|| qualified_repo_host(value))
}

fn qualified_repo_host(value: &str) -> Option<String> {
    let parts = value.split('/').collect::<Vec<_>>();
    if parts.len() >= 3 && !parts[0].is_empty() && !parts[1].is_empty() && !parts[2].is_empty() {
        return Some(parts[0].to_string());
    }
    None
}

fn repository_url_host(value: &str) -> Option<String> {
    if let Some(rest) = value
        .strip_prefix("https://")
        .or_else(|| value.strip_prefix("http://"))
        .or_else(|| value.strip_prefix("ssh://"))
        .or_else(|| value.strip_prefix("git://"))
        .or_else(|| value.strip_prefix("git+ssh://"))
    {
        let authority = rest.split('/').next()?;
        let host = authority.rsplit('@').next()?.split(':').next()?;
        return (!host.is_empty()).then(|| host.to_string());
    }

    let (user_host, path) = value.split_once(':')?;
    if path.is_empty() || user_host.contains('/') {
        return None;
    }
    let host = user_host.rsplit('@').next()?;
    (!host.is_empty() && (user_host.contains('@') || host.contains('.'))).then(|| host.to_string())
}

fn non_empty_env(name: &str) -> Result<Option<String>, String> {
    let Some(value) = std::env::var_os(name) else {
        return Ok(None);
    };
    let value = value
        .into_string()
        .map_err(|_| format!("{name} is not valid UTF-8"))?;
    if value.is_empty() {
        return Ok(None);
    }
    Ok(Some(value))
}

#[cfg(test)]
mod tests {
    use super::{qualified_repo_host, repository_url_host};

    #[test]
    fn recognizes_only_repository_url_shapes() {
        assert_eq!(
            repository_url_host("https://github.com/org/repo.git"),
            Some("github.com".to_string())
        );
        assert_eq!(
            repository_url_host("git@github.com:org/repo.git"),
            Some("github.com".to_string())
        );
        assert_eq!(repository_url_host("a PR body https://evil.example"), None);
    }

    #[test]
    fn recognizes_host_qualified_repository_references() {
        assert_eq!(
            qualified_repo_host("github.com/org/repo"),
            Some("github.com".to_string())
        );
        assert_eq!(qualified_repo_host("org/repo"), None);
        assert_eq!(
            qualified_repo_host("feature/team/change"),
            Some("feature".to_string())
        );
    }
}
