use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    pub username: String,
    pub origin: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountSelection {
    Stock,
    Configured(Account),
    Override(String),
}

pub fn resolve_account(cwd: &Path) -> Result<AccountSelection, String> {
    if let Some(value) = std::env::var_os("GH_AUTO_SWITCHER_ACCOUNT") {
        let account = value
            .into_string()
            .map_err(|_| "GH_AUTO_SWITCHER_ACCOUNT is not valid UTF-8".to_string())?;
        validate_account(&account)?;
        return Ok(AccountSelection::Override(account));
    }

    let Some(username) = get_value(cwd, "github.account")? else {
        return Ok(AccountSelection::Stock);
    };

    validate_account(&username)?;
    let origin = get_origin(cwd, "github.account")?;
    Ok(AccountSelection::Configured(Account { username, origin }))
}

pub fn get_value(cwd: &Path, key: &str) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .args(["config", "--get", "--null", key])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("failed to run git config: {error}"))?;

    if output.status.code() == Some(1) {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(format!(
            "git config failed while reading {key} (exit {})",
            exit_description(&output)
        ));
    }

    let value = output
        .stdout
        .split(|byte| *byte == 0)
        .next()
        .ok_or_else(|| format!("git config returned no value for {key}"))?;
    let value = String::from_utf8(value.to_vec())
        .map_err(|_| format!("git config returned invalid UTF-8 for {key}"))?;
    Ok(Some(value.trim().to_string()))
}

pub fn get_origin(cwd: &Path, key: &str) -> Result<Option<String>, String> {
    let output = Command::new("git")
        .args(["config", "--show-origin", "--get", "--null", key])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("failed to run git config: {error}"))?;

    if output.status.code() == Some(1) {
        return Ok(None);
    }
    if !output.status.success() {
        return Err(format!(
            "git config failed while locating {key} (exit {})",
            exit_description(&output)
        ));
    }

    let fields: Vec<&[u8]> = output.stdout.split(|byte| *byte == 0).collect();
    let origin = fields
        .first()
        .filter(|field| !field.is_empty())
        .ok_or_else(|| format!("git config returned no origin for {key}"))?;
    String::from_utf8(origin.to_vec())
        .map(Some)
        .map_err(|_| format!("git config returned an invalid origin for {key}"))
}

pub fn remote_urls(cwd: &Path) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["config", "--get-regexp", "^remote\\..*\\.url$"])
        .current_dir(cwd)
        .output()
        .map_err(|error| format!("failed to run git config: {error}"))?;

    if output.status.code() == Some(1) {
        return Ok(Vec::new());
    }
    if !output.status.success() {
        return Err(format!(
            "git config failed while reading remotes (exit {})",
            exit_description(&output)
        ));
    }

    let text = String::from_utf8(output.stdout)
        .map_err(|_| "git config returned invalid UTF-8 for remote URLs".to_string())?;
    Ok(text
        .lines()
        .filter_map(|line| {
            line.split_once(char::is_whitespace)
                .map(|(_, value)| value.trim())
        })
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect())
}

pub fn validate_account(account: &str) -> Result<(), String> {
    if account.is_empty() {
        return Err("github.account is empty".to_string());
    }
    if account.len() > 39
        || !account
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        return Err(format!(
            "invalid GitHub account {account:?}; expected 1-39 ASCII letters, numbers, or hyphens"
        ));
    }
    Ok(())
}

fn exit_description(output: &std::process::Output) -> String {
    output
        .status
        .code()
        .map_or_else(|| "signal".to_string(), |code| code.to_string())
}

#[cfg(test)]
mod tests {
    use super::validate_account;

    #[test]
    fn accepts_github_login_shape() {
        assert!(validate_account("carter-hp").is_ok());
    }

    #[test]
    fn rejects_empty_or_unsafe_accounts() {
        assert!(validate_account("").is_err());
        assert!(validate_account("carter/hp").is_err());
        assert!(validate_account("account with spaces").is_err());
    }
}
