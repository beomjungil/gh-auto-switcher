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
- one or more authenticated GitHub accounts;
- Bash, Zsh, or Fish if you want normal `gh ...` commands to switch accounts
  transparently.

Install the prebuilt extension:

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

The current release includes Linux amd64, macOS amd64, and macOS arm64
binaries. GitHub CLI selects the matching binary for your platform.

## Authenticate the accounts

Authenticate every GitHub account you want to use with the normal GitHub CLI
commands. GitHub CLI keeps these profiles; this extension does not modify them.

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

Run the login flow again when adding another account. Confirm that the logins
are the GitHub usernames you expect:

```sh
gh auth status --hostname github.com
```

## How an account is selected

For each command, the launcher reads Git's effective configuration in the
current working directory. The first applicable rule wins:

1. `GH_AUTO_SWITCHER_ACCOUNT` selects an account for one process;
2. `github.account` explicitly selects an account;
3. `user.name` selects an account when it is a valid GitHub username and the
   same authenticated `gh` profile exists;
4. if no matching profile exists, the real `gh` runs unchanged.

`github.account` is optional. Use it when your Git commit identity is not the
same as your GitHub login.

For example, this needs no extra setting when the Git identity is also the
GitHub username:

```ini
[user]
    name = beomjungil
```

If the displayed Git identity differs from the GitHub username, add the
explicit account key:

```ini
[user]
    name = Beom Jungil

[github]
    account = beomjungil
```

Git evaluates local, global, and `includeIf` configuration normally. This also
works for repository-specific account selection:

```ini
# ~/.gitconfig
[user]
    name = beomjungil

[includeIf "hasconfig:remote.*.url:https://github.com/healingpaper-solution/**"]
    path = ~/.gitconfig-healingpaper
```

```ini
# ~/.gitconfig-healingpaper
[user]
    name = carter-hp
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

## Choose how to run commands

### Option A: transparent normal `gh` commands

Use this when you want commands such as `gh pr list` to select an account
based on the current repository. A shell hook is required because the GitHub
CLI extension itself cannot replace the `gh` executable.

The hook does not edit your shell startup file. Add the command for your shell
to your startup file, or evaluate it only in the current shell.

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

After the hook is active, use ordinary commands:

```sh
gh auto-switcher status
gh auto-switcher doctor
gh pr list
gh issue list
gh api repos/OWNER/REPOSITORY
```

The hook is only a thin function. It does not change directories, export a
token in the parent shell, or change GitHub CLI's active account.

### Option B: explicit launcher commands without a shell hook

A shell hook is not needed when you call the launcher explicitly:

```sh
gh auto-switcher exec -- pr list
gh auto-switcher exec -- api repos/OWNER/REPOSITORY
```

For a standalone binary installed on `PATH`, use:

```sh
gh-auto-switcher exec -- pr list
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
printing a token:

```sh
gh auto-switcher status
```

`doctor` verifies that the selected account can provide a token through the
normal GitHub CLI credential store. It does not make an API request:

```sh
gh auto-switcher doctor
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
