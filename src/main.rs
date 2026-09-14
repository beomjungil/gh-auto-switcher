use std::env;
use std::ffi::{OsStr, OsString};

use gh_auto_switcher::{auth, gitconfig, runner, shell};

fn main() {
    if let Err(error) = run() {
        eprintln!("gh-auto-switcher: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args = env::args_os().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        print_usage();
        return Ok(());
    }

    if args[0] == OsStr::new("exec") {
        let wrapped = parse_exec_args(&args[1..]);
        return dispatch_wrapped(&wrapped);
    }

    run_management(&management_args(&args))
}

fn parse_exec_args(args: &[OsString]) -> Vec<OsString> {
    args.iter()
        .position(|arg| arg == OsStr::new("--"))
        .map(|index| args[index + 1..].to_vec())
        .unwrap_or_else(|| args.to_vec())
}

fn dispatch_wrapped(args: &[OsString]) -> Result<(), String> {
    if runner::is_management_command(args) {
        return run_management(&management_args(args));
    }
    runner::execute(args)
}

fn management_args(args: &[OsString]) -> Vec<OsString> {
    runner::first_command_index(args)
        .filter(|index| args[*index] == OsStr::new("auto-switcher"))
        .map_or_else(|| args.to_vec(), |index| args[index + 1..].to_vec())
}

fn run_management(args: &[OsString]) -> Result<(), String> {
    match args.first().and_then(|arg| arg.to_str()) {
        Some("status") => status(),
        Some("doctor") => doctor(),
        Some("shell-hook") => shell_hook(args.get(1)),
        Some("help" | "--help") | None => {
            print_usage();
            Ok(())
        }
        Some(command) => Err(format!(
            "unknown command {command:?}; run `gh auto-switcher help`"
        )),
    }
}

fn status() -> Result<(), String> {
    let cwd = env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    let routing = runner::resolve_routing(&cwd, &[])?;

    println!("working-directory: {}", cwd.display());
    match &routing.account {
        None => {
            println!("account: <not used by explicit token>");
            println!("account-source: explicit environment token");
        }
        Some(gitconfig::AccountSelection::Stock) => {
            println!("account: <not configured>");
            println!("account-source: stock gh");
        }
        Some(gitconfig::AccountSelection::Configured(account)) => {
            println!("account: {}", account.username);
            println!(
                "account-origin: {}",
                account.origin.as_deref().unwrap_or("unknown")
            );
            println!("account-source: Git config");
        }
        Some(gitconfig::AccountSelection::Override(account)) => {
            println!("account: {account}");
            println!("account-origin: GH_AUTO_SWITCHER_ACCOUNT");
            println!("account-source: explicit override");
        }
    }

    match &routing.host {
        Some(host) => {
            println!("target-host: {}", host.host);
            println!("target-host-source: {}", host.source);
        }
        None => println!("target-host: <not resolved; stock gh>"),
    }
    match routing.credential_mode {
        runner::CredentialMode::Stock => println!("mode: stock gh"),
        runner::CredentialMode::ExplicitEnvironment => {
            println!("mode: explicit environment token (Git account is not used)")
        }
        runner::CredentialMode::ProcessLocalAccount(_) => {
            println!("mode: process-local account token")
        }
    }
    Ok(())
}

fn doctor() -> Result<(), String> {
    let cwd = env::current_dir()
        .map_err(|error| format!("failed to resolve current directory: {error}"))?;
    let routing = runner::resolve_routing(&cwd, &[])?;

    match routing.credential_mode {
        runner::CredentialMode::Stock => {
            println!("Git account: not configured (stock gh behavior will be used)");
        }
        runner::CredentialMode::ExplicitEnvironment => {
            println!("GitHub authentication: explicit environment token");
            if let Some(host) = routing.host {
                println!("Target host: {}", host.host);
            }
        }
        runner::CredentialMode::ProcessLocalAccount(account) => {
            let real_gh = runner::find_real_gh()?;
            auth::token_for(&real_gh, &account)?;
            println!("Git account: {account}");
            println!("GitHub authentication: available");
            if let Some(host) = routing.host {
                println!("Target host: {}", host.host);
            }
        }
    }
    Ok(())
}

fn shell_hook(value: Option<&OsString>) -> Result<(), String> {
    let shell = value
        .map(|value| {
            value
                .to_str()
                .and_then(shell::Shell::parse)
                .ok_or_else(|| "shell must be valid UTF-8".to_string())
        })
        .unwrap_or_else(|| Ok(shell::Shell::detect()))?;
    print!("{}", shell::hook(shell));
    Ok(())
}

fn print_usage() {
    println!(
        "gh-auto-switcher\n\nUsage:\n  gh auto-switcher status\n  gh auto-switcher doctor\n  gh auto-switcher shell-hook [bash|zsh|fish]\n  gh-auto-switcher exec -- [normal gh arguments]\n\nSet [github] account in Git config to select an authenticated gh account."
    );
}
