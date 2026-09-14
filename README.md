# gh-auto-switcher

Use the normal [GitHub CLI (`gh`)](https://cli.github.com/) while selecting a
stored GitHub account from Git's effective configuration for the current
working context.

[한국어 문서](README.ko.md) · [English usage guide](docs/en/usage.md) ·
[한국어 사용법](docs/ko/usage.md) · [Architecture / 아키텍처](docs/en/architecture.md)

## What it does

`gh-auto-switcher` is a launcher. For a repository whose effective
Git configuration contains `github.account = work-github-username`, a normal command such
as `gh pr list` retrieves that account's stored token and runs the real `gh`
with the token in the child process environment.

It does not switch the globally active GitHub account, edit `hosts.yml`, write
token files, or maintain a second repository-to-account database.

## Features

- Selects an authenticated GitHub account from Git's effective
  `github.account` configuration;
- injects the selected token only into the child `gh` process;
- works with a transparent Bash, Zsh, or Fish shell hook;
- leaves GitHub CLI's account store and authentication-management commands under
  stock `gh` control.

Automatic account injection is limited to an unambiguous `github.com` target.
An applicable caller token may pass through for one unsupported Enterprise host,
but conflicting host signals are rejected before credential precedence is
applied. Host inference is conservative and does not try to understand every
`gh` alias, extension, or API payload.

Read the [routing and safety details](docs/en/usage.md#routing-and-safety) before
using the launcher in automation.

## Prerequisites

- Git with support for the configuration patterns used in your examples;
- GitHub CLI (`gh`) installed and authenticated for the accounts you select;
- Bash, Zsh, or Fish for transparent `gh` commands;
- Cargo/Rust only when using the source installation or local development path.

## Quick start

You can use either a prebuilt GitHub CLI extension or a source installation.

### Option A: GitHub CLI extension

Install the extension from GitHub. GitHub CLI selects the matching prebuilt
release asset for the current platform:

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

The release workflow is configured for `v*` tags and produces Linux amd64,
macOS amd64, and macOS arm64 assets. If no release has been published yet,
GitHub CLI falls back to the source extension and Cargo/Rust is required.

### Option B: standalone source executable

Install the launcher on `PATH`:

```sh
cd /path/to/gh-auto-switcher
cargo install --path .
gh-auto-switcher help
```

Authenticate the accounts in the normal GitHub CLI store. Repeat the login
flow as needed for each account:

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

Set the account convention in a Git config scope that matches your workflow:

```sh
git config --global github.account personal-github-username
```

Source the hook for your current shell:

```sh
# Bash, after `gh extension install beomjungil/gh-auto-switcher`
eval "$(gh auto-switcher shell-hook bash)"

# Zsh, after `gh extension install beomjungil/gh-auto-switcher`
eval "$(gh auto-switcher shell-hook zsh)"

# Fish, after `gh extension install beomjungil/gh-auto-switcher`
source (gh auto-switcher shell-hook fish | psub)

# Or use the standalone executable from Option B:
# eval "$(gh-auto-switcher shell-hook bash)"
```

Check the resolved account without printing a token, then use ordinary
commands:

```sh
gh auto-switcher status
gh auto-switcher doctor
gh pr list
gh api repos/OWNER/REPOSITORY
```

For conditional includes, per-command overrides, shell removal, and
troubleshooting, see the [English usage guide](docs/en/usage.md) or
[한국어 사용법](docs/ko/usage.md).

## Documentation map

| Need | English | 한국어 |
| --- | --- | --- |
| Get started | This README | [README.ko.md](README.ko.md) |
| Install and operate the launcher | [Usage](docs/en/usage.md) | [사용법](docs/ko/usage.md) |
| Understand design and guarantees | [Architecture](docs/en/architecture.md) | [아키텍처](docs/ko/architecture.md) |

## Security model

- The selected token is requested through the real `gh auth token` command.
- Token output is captured privately and is never printed or persisted.
- The token is supplied only to the child `gh` process as `GH_TOKEN`.
- Authentication-management commands such as `gh auth login`, `logout`,
  `switch`, `refresh`, `token`, and `status` use stock `gh` behavior.
- A configured account that cannot provide a token fails clearly; it does not
  silently fall back to another account.

See [Architecture: credential lifecycle](docs/en/architecture.md#credential-lifecycle)
for the complete flow.

## Development

For local extension development, run the install command from the project
directory. GitHub CLI manages it as a link to that checkout:

```sh
cd /path/to/gh-auto-switcher
gh extension install .
```

Source checks and tests:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
python3 -m pip install --user PyYAML==6.0.2
python3 tests/release_workflow_test.py
```

The tests use real Git, isolated temporary configuration, fake `gh`
executables, actual shell processes, an isolated local `gh extension install`
run, and a prebuilt-extension execution path. They do not use live credentials or
modify the user's Git, shell, or GitHub CLI extension configuration.
