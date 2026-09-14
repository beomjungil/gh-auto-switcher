use serde::Deserialize;
use serde_yaml::Value;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Deserialize)]
struct HostConfig {
    user: Option<String>,
    #[serde(default)]
    users: Option<BTreeMap<String, Value>>,
}

pub fn profile_exists(account: &str) -> Result<bool, String> {
    let hosts_path = hosts_path()?;
    let contents = match fs::read_to_string(&hosts_path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => {
            return Err(format!(
                "failed to read GitHub authentication profiles: {error}"
            ));
        }
    };

    let profiles = github_profile_names(&contents)
        .map_err(|_| "failed to parse GitHub authentication profiles".to_string())?;
    Ok(profiles
        .iter()
        .any(|profile| profile.eq_ignore_ascii_case(account)))
}

fn hosts_path() -> Result<PathBuf, String> {
    let config_dir = non_empty_env_path("GH_CONFIG_DIR");
    let xdg_config_home = non_empty_env_path("XDG_CONFIG_HOME");
    #[cfg(windows)]
    let default_config_dir = non_empty_env_path("APPDATA").map(|path| path.join("GitHub CLI"));
    #[cfg(not(windows))]
    let default_config_dir = non_empty_env_path("HOME").map(|path| path.join(".config/gh"));

    resolve_hosts_path(config_dir, xdg_config_home, default_config_dir)
}

fn non_empty_env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn resolve_hosts_path(
    config_dir: Option<PathBuf>,
    xdg_config_home: Option<PathBuf>,
    default_config_dir: Option<PathBuf>,
) -> Result<PathBuf, String> {
    config_dir
        .map(|path| path.join("hosts.yml"))
        .or_else(|| xdg_config_home.map(|path| path.join("gh/hosts.yml")))
        .or_else(|| default_config_dir.map(|path| path.join("hosts.yml")))
        .ok_or_else(|| "cannot locate GitHub authentication profiles".to_string())
}

fn github_profile_names(contents: &str) -> Result<Vec<String>, serde_yaml::Error> {
    let hosts: BTreeMap<String, HostConfig> = serde_yaml::from_str(contents)?;
    let Some(github) = hosts.get("github.com") else {
        return Ok(Vec::new());
    };

    let mut profiles: Vec<String> = github
        .users
        .as_ref()
        .map(|users| users.keys().cloned().collect())
        .unwrap_or_default();
    if let Some(user) = github.user.as_deref() {
        profiles.push(user.to_string());
    }
    Ok(profiles)
}

pub fn token_for(real_gh: &Path, account: &str) -> Result<String, String> {
    let output = Command::new(real_gh)
        .args([
            "auth",
            "token",
            "--hostname",
            "github.com",
            "--user",
            account,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .env_remove("GH_TOKEN")
        .env_remove("GITHUB_TOKEN")
        .env_remove("GH_ENTERPRISE_TOKEN")
        .env_remove("GITHUB_ENTERPRISE_TOKEN")
        .output()
        .map_err(|_| "failed to run gh auth token".to_string())?;

    if !output.status.success() {
        return Err(format!(
            "no GitHub authentication found for account {account:?}; run `gh auth login --hostname github.com`"
        ));
    }

    let token = String::from_utf8(output.stdout)
        .map_err(|_| "gh auth token returned invalid UTF-8".to_string())?;
    let token = token.trim().to_string();
    if token.is_empty() {
        return Err(format!(
            "no GitHub authentication found for account {account:?}; run `gh auth login --hostname github.com`"
        ));
    }
    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::{github_profile_names, resolve_hosts_path};
    use std::path::PathBuf;

    #[test]
    fn reads_profiles_from_current_and_stored_users() {
        let profiles = github_profile_names(
            "github.com:\n  users:\n    beomjungil: {}\n    carter-hp: null\n  user: carter-hp\n",
        )
        .expect("parse hosts");

        assert_eq!(profiles, ["beomjungil", "carter-hp", "carter-hp"]);
    }

    #[test]
    fn reads_legacy_current_user_profile() {
        assert_eq!(
            github_profile_names("github.com:\n  user: carter-hp\n").expect("parse hosts"),
            ["carter-hp"]
        );
    }

    #[test]
    fn ignores_other_hosts() {
        let profiles = github_profile_names(
            "enterprise.example.com:\n  user: carter-hp\ngithub.com:\n  users:\n    beomjungil:\n",
        )
        .expect("parse hosts");

        assert_eq!(profiles, ["beomjungil"]);
    }

    #[test]
    fn rejects_malformed_hosts() {
        assert!(github_profile_names("github.com: [").is_err());
    }

    #[test]
    fn compares_profile_names_without_case_sensitivity() {
        let profiles = github_profile_names("github.com:\n  users:\n    Carter-HP: {}\n")
            .expect("parse hosts");

        assert!(profiles
            .iter()
            .any(|profile| profile.eq_ignore_ascii_case("carter-hp")));
    }

    #[test]
    fn resolves_config_directory_precedence_and_empty_values() {
        assert_eq!(
            resolve_hosts_path(
                Some(PathBuf::from("/custom")),
                Some(PathBuf::from("/xdg")),
                Some(PathBuf::from("/home/.config/gh")),
            )
            .expect("custom config path"),
            PathBuf::from("/custom/hosts.yml")
        );
        assert_eq!(
            resolve_hosts_path(
                None,
                Some(PathBuf::from("/xdg")),
                Some(PathBuf::from("/home/.config/gh")),
            )
            .expect("xdg config path"),
            PathBuf::from("/xdg/gh/hosts.yml")
        );
        assert_eq!(
            resolve_hosts_path(None, None, Some(PathBuf::from("/home/.config/gh")))
                .expect("default config path"),
            PathBuf::from("/home/.config/gh/hosts.yml")
        );
        assert!(resolve_hosts_path(None, None, None).is_err());
    }
}
