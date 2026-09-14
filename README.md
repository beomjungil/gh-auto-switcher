# gh-auto-switcher

Use the GitHub CLI (`gh`) with the GitHub account that matches your current
Git repository context.

`gh` can store more than one account for the same GitHub host, but its normal
account selection is global for that host. This extension keeps account
selection local to each command: Git chooses the current identity, and the
launcher gives the real `gh` process only that account's token.

## Install

Requirements:

- GitHub CLI (`gh`);
- Git;
- one or more authenticated GitHub accounts;
- Bash, Zsh, or Fish if you want normal `gh ...` commands to switch accounts
  transparently.

Choose one launcher installation method. The account, Git configuration, and
shell-hook steps below are common to both methods.

### Option A: prebuilt GitHub CLI extension (recommended)

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

The current release includes Linux amd64, macOS amd64, and macOS arm64
binaries. GitHub CLI selects the matching binary for your platform.

### Option B: standalone source executable

Use this when you want `gh-auto-switcher` directly on `PATH`. This method
requires Cargo/Rust:

```sh
cd /path/to/gh-auto-switcher
cargo install --path .
gh-auto-switcher help
```

## Authenticate the accounts

Authenticate every GitHub account you want to use with the normal GitHub CLI
commands. GitHub CLI keeps these profiles; this extension does not modify them.
Repeat the login flow for each account you want to select:

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

Confirm that the stored logins are the GitHub usernames you expect:

```sh
gh auth status --hostname github.com
```

## Configure account selection

For each command, the launcher reads Git's effective configuration in the
current working directory. The first applicable account rule wins:

1. `GH_AUTO_SWITCHER_ACCOUNT` selects an account for one process;
2. `github.account` explicitly selects an account;
3. `user.name` selects an account when it is a valid GitHub username and the
   same authenticated `gh` profile exists;
4. if no matching profile exists, the real `gh` runs unchanged.

Replace example values such as `personal-github-username` and
`work-github-username` with the actual GitHub login names from `gh auth status`.

When the effective `user.name` is already the GitHub username of an
authenticated profile, no `github.account` setting is needed:

```ini
[user]
    name = personal-github-username
```

This also works with conditional Git identities, so different repositories can
select different authenticated profiles without any `github.account` setting:

```ini
# ~/.gitconfig
[user]
    name = personal-github-username

[includeIf "hasconfig:remote.*.url:https://github.com/customer/**"]
    path = ~/.gitconfig-customer
```

```ini
# ~/.gitconfig-customer
[user]
    name = work-github-username
```

When the Git commit identity is a display name or otherwise differs from the
GitHub login, use an explicit `github.account` mapping. Prefer the narrowest
scope that matches the repositories that need it.

For one repository, use local configuration:

```sh
cd /path/to/customer-repository
git config --local github.account work-github-username
```

For a group of repositories, use a conditional include:

```ini
# ~/.gitconfig
[includeIf "hasconfig:remote.*.url:https://github.com/customer/**"]
    path = ~/.gitconfig-customer
```

```ini
# ~/.gitconfig-customer
[github]
    account = work-github-username
```

Use a global setting only when the same account should be the explicit default
for every repository:

```sh
git config --global github.account personal-github-username
```

`github.account` is explicit and takes precedence over `user.name`. Therefore,
do not set it globally if different repositories should select accounts from
their different `user.name` values. Local or conditional `github.account`
settings can provide narrower overrides when needed.

If the displayed Git identity should still be used for commits, keep `user.name`
and set only the account mapping:

```ini
[user]
    name = Beom Jungil

[github]
    account = personal-github-username
```

Check the effective values and their origins with:

```sh
git config --get user.name
git config --get github.account
git config --show-origin --get user.name
git config --show-origin --get github.account
gh auth status --hostname github.com
```

An invalid display name or a `user.name` without a matching authenticated
profile does not cause a different account to be guessed. The normal `gh`
behavior is preserved. An explicit `github.account` or
`GH_AUTO_SWITCHER_ACCOUNT` that cannot be authenticated fails clearly instead
of silently falling back.

## Enable transparent `gh` commands

Use a shell hook when you want ordinary commands such as `gh pr list` to select
an account based on the current repository. The GitHub CLI extension cannot
replace the `gh` executable by itself. `gh extension install` also does not
modify your shell startup file.

Install the launcher first, then add exactly one hook command for that
installation to `~/.bashrc`, `~/.zshrc`, or Fish's
`~/.config/fish/config.fish`. After adding it to a startup file, open a new
shell; to activate it immediately, evaluate or source the same command in the
current shell.

### GitHub CLI extension

Bash:

```sh
eval "$(gh auto-switcher shell-hook bash)"
```

Zsh:

```sh
eval "$(gh auto-switcher shell-hook zsh)"
```

Fish:

```fish
source (gh auto-switcher shell-hook fish | psub)
```

### Standalone executable

Bash:

```sh
eval "$(gh-auto-switcher shell-hook bash)"
```

Zsh:

```sh
eval "$(gh-auto-switcher shell-hook zsh)"
```

Fish:

```fish
source (gh-auto-switcher shell-hook fish | psub)
```

Use only the commands matching your installation. After the hook is active,
ordinary commands go through the launcher:

```sh
gh auto-switcher status
gh auto-switcher doctor
gh pr list
gh issue list
gh api repos/OWNER/REPOSITORY
```

The hook is only a thin function. It does not change directories, export a
token in the parent shell, or change GitHub CLI's active account.

## Run commands without a shell hook

A shell hook is not needed when you call the launcher explicitly:

```sh
gh auto-switcher exec -- pr list
gh auto-switcher exec -- api repos/OWNER/REPOSITORY
```

For a standalone binary installed on `PATH`, use:

```sh
gh-auto-switcher exec -- pr list
gh-auto-switcher exec -- api repos/OWNER/REPOSITORY
```

Without the hook or an explicit `exec` call, a normal `gh pr list` is just the
stock GitHub CLI and this extension is not involved.

For commands outside a repository or for a one-off choice, use the process
override:

```sh
GH_AUTO_SWITCHER_ACCOUNT=carter-hp gh auto-switcher exec -- pr list
```

## Inspect the selected route

`status` shows the account source, target host, and credential mode without
printing a token. Use the command matching your installation:

```sh
# GitHub CLI extension
gh auto-switcher status

# Standalone executable
gh-auto-switcher status
```

`doctor` verifies that the selected account can provide a token through the
normal GitHub CLI credential store. It does not make an API request:

```sh
# GitHub CLI extension
gh auto-switcher doctor

# Standalone executable
gh-auto-switcher doctor
```

## What the launcher changes

- It supplies the selected token only to the child `gh` process.
- It never changes GitHub CLI's globally active account.
- It never edits `hosts.yml`, stores tokens, or creates a second account mapping
  database.
- Explicit applicable `GH_TOKEN`/`GITHUB_TOKEN` values are preserved and take
  precedence over configured account selection.
- Authentication-management commands such as `gh auth login`, `logout`,
  `switch`, `refresh`, `token`, and `status` use the real `gh` behavior.
- Automatic token injection is limited to an unambiguous `github.com` target.
  Conflicting host signals are rejected rather than guessed.

A direct executable path such as `/opt/homebrew/bin/gh pr list`, a script that
does not source the hook, and another shell function that bypasses the launcher
will not be intercepted.

## Troubleshooting

See the [English usage guide](docs/en/usage.md) for detailed routing, shell,
conditional-include, host, and failure behavior. The [architecture guide](docs/en/architecture.md)
describes the security boundaries.

Useful checks:

```sh
type -a gh
gh auto-switcher status
git config --show-origin --get user.name
git config --show-origin --get github.account
gh auth status --hostname github.com
```

If the hook is not listed before the real `gh` in `type -a gh`, evaluate the
hook again in the current shell. If no matching `user.name` profile exists,
that is expected to result in normal stock `gh` behavior.

## Contribute

Clone the repository and run the local checks:

```sh
git clone https://github.com/beomjungil/gh-auto-switcher.git
cd gh-auto-switcher
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
python3 tests/release_workflow_test.py
```

To test the extension entrypoint locally:

```sh
gh extension install .
gh auto-switcher help
```

The test suite uses isolated Git and GitHub CLI fixtures and never changes your
personal Git configuration, credentials, shell startup files, or installed
extension configuration.
