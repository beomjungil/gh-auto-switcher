use std::path::Path;
use std::process::{Command, Stdio};

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
