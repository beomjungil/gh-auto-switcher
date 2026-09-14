# Usage

This guide is the operational reference for `gh-auto-switcher`. It is written
as a set of how-to instructions: use the [architecture guide](architecture.md)
when you need to understand why the launcher makes a particular decision.

[한국어 사용법](../ko/usage.md) · [English README](../../README.md) ·
[한국어 README](../../README.ko.md)

## 1. Install the launcher

`gh-auto-switcher` supports a prebuilt GitHub CLI extension and source
installation. It does not edit shell startup files for you.

### Remote GitHub CLI extension

Install the extension from its distribution repository:

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

When a matching release asset exists, GitHub CLI installs and runs that prebuilt
binary. The release workflow is configured to publish Linux amd64, macOS amd64,
and macOS arm64 assets. Prebuilt installation does not require Cargo/Rust.

Before the first release, GitHub CLI falls back to the repository's root
`gh-auto-switcher` source entrypoint, which performs this build on each
invocation. If a release exists but does not contain an asset for the current
platform, installation fails rather than falling back to source:

```sh
cargo build --release --locked --manifest-path /path/to/gh-auto-switcher/Cargo.toml
```

The build artifact is then executed with the original working directory and
arguments. A source change is picked up on the next invocation. This source
extension requires Cargo/Rust wherever it is executed.

### Local GitHub CLI extension for development

The project directory is already named according to GitHub CLI's local extension
convention (`gh-<extension-name>`), and the root executable has the required
matching name. For local development only, run the install command from the
project directory:

```sh
cd /path/to/gh-auto-switcher
gh extension install .
```

GitHub CLI manages the local extension as a link to the checkout. It does not
build the project during installation.

### Standalone executable

From the project directory, install the launcher on `PATH`:

```sh
cd /path/to/gh-auto-switcher
cargo install --path .
```

Confirm that the installed executable is available:

```sh
command -v gh-auto-switcher
gh-auto-switcher help
```

The currently tested source-build toolchain is Rust 1.93.1. The project does
not currently claim an older minimum supported Rust version.

To remove a Cargo-installed standalone binary:

```sh
cargo uninstall gh-auto-switcher
```

To remove the GitHub CLI extension:

```sh
gh extension remove auto-switcher
```

Removing the extension does not remove the source checkout or a separately
installed standalone binary.

If the real GitHub CLI is not discoverable through `PATH`, provide its path for
one process or for a shell session:

```sh
GH_AUTO_SWITCHER_REAL_GH=/opt/homebrew/bin/gh gh-auto-switcher status
export GH_AUTO_SWITCHER_REAL_GH=/opt/homebrew/bin/gh
```

The launcher checks that this path is a file with execute permission and rejects
a path that resolves to the launcher itself.

### Upgrade an extension

For a remote extension, upgrade through GitHub CLI:

```sh
gh extension upgrade auto-switcher
```

For a local source extension, update the checkout and invoke the extension again:

```sh
cd /path/to/gh-auto-switcher
git pull
gh auto-switcher help
```

The local source entrypoint rebuilds the release artifact before execution.

## 2. Authenticate accounts with stock `gh`

Authenticate accounts using the normal GitHub CLI flow. This project does not
manage the GitHub CLI credential store.

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

Repeat the login flow for each account that you intend to select. The account
name used by this project must be the login accepted by `gh auth token`.

The launcher retrieves a selected account's token using the equivalent of:

```sh
gh auth token --hostname github.com --user ACCOUNT
```

Token retrieval uses the real `gh`, not the shell function installed in the
next step.

## 3. Install a shell hook

The hook defines a thin `gh` function. It forwards ordinary arguments to the
launcher; it does not run on `cd`, export a token, or change the parent shell's
working directory.

Add one command to the shell startup file after installing the binary, or run
it temporarily in the current shell.

### Bash

```sh
# After `gh extension install beomjungil/gh-auto-switcher`
eval "$(gh auto-switcher shell-hook bash)"

# After standalone `cargo install --path .`
eval "$(gh-auto-switcher shell-hook bash)"
```

### Zsh

```sh
# After `gh extension install beomjungil/gh-auto-switcher`
eval "$(gh auto-switcher shell-hook zsh)"

# After standalone `cargo install --path .`
eval "$(gh-auto-switcher shell-hook zsh)"
```

### Fish

```fish
# After `gh extension install beomjungil/gh-auto-switcher`
source (gh auto-switcher shell-hook fish | psub)

# After standalone `cargo install --path .`
source (gh-auto-switcher shell-hook fish | psub)
```

The generated hook first checks whether a standalone `gh-auto-switcher` is
available on `PATH`. If so, it calls that executable; otherwise it calls
`command gh auto-switcher exec -- ...` to re-enter the installed extension
without recursion. This lets the same hook work with both prebuilt extension
binaries and standalone installations. The launcher discovers the real `gh`
from `PATH` at each invocation and excludes itself; it does not capture or cache
that path. Use `GH_AUTO_SWITCHER_REAL_GH` when the lookup is not suitable.

Verify that the function is active:

```sh
type -a gh
```

The exact output varies by shell, but the function should appear before the
real `gh` executable.

### Remove the hook

Remove the `eval`/`source` line from the startup file and start a new shell.
For the current shell, use the command for that shell:

```sh
# Bash
unset -f gh

# Zsh
unfunction gh
```

```fish
functions --erase gh
```

If another tool installed a `gh` function before this one, save or restore it
rather than erasing it blindly.

## 4. Configure the account convention

The launcher reads the effective value of `github.account` through Git itself:

```sh
git config --get github.account
```

Set a default account globally:

```sh
git config --global github.account personal-github-username
```

Or set the account only for one repository:

```sh
cd ~/src/customer-repository
git config --local github.account work-github-username
```

The value must be 1–39 ASCII letters, numbers, or hyphens. Empty values,
slashes, spaces, and other malformed values fail instead of selecting a
fallback account.

### Conditional includes

Git evaluates the configuration. The launcher does not reimplement Git's
condition syntax, so normal include and precedence behavior is preserved.

A default account can be combined with a repository-path condition:

```ini
# ~/.gitconfig
[github]
    account = personal-github-username

[includeIf "gitdir:~/src/customer/**"]
    path = ~/.gitconfig-customer
```

```ini
# ~/.gitconfig-customer
[github]
    account = work-github-username
```

A remote URL condition can select an account for a company organization:

```ini
[includeIf "hasconfig:remote.*.url:https://github.com/customer/**"]
    path = ~/.gitconfig-customer
```

Other Git-supported conditions, including `gitdir`, `gitdir/i`, `onbranch`,
nested includes, linked worktrees, and normal local/global precedence are
resolved by the installed Git version. The integration suite exercises these
patterns with real Git.

Do not use `user.name` or `user.email` as the account setting. They are not
used to infer a GitHub login.

## 5. Run normal commands

After the hook is active and `github.account` resolves to a valid account,
ordinary commands can be typed normally:

```sh
gh pr list
gh api repos/OWNER/REPOSITORY
gh issue list --limit 10
gh repo view OWNER/REPOSITORY
```

Each invocation re-evaluates Git configuration from the current working
context. A branch or remote change is considered by the next command without a
`cd` hook.

For a command outside a repository or targeting another repository, select an
account explicitly for that process:

```sh
GH_AUTO_SWITCHER_ACCOUNT=work-github-username gh repo clone customer/example
GH_AUTO_SWITCHER_ACCOUNT=personal-github-username gh --repo personal-github-username/example pr list
```

The override is an environment variable, so it does not persist in the
launcher or Git configuration.

## Management commands

These commands can be used through the shell function:

```sh
gh auto-switcher status
gh auto-switcher doctor
gh auto-switcher shell-hook bash
```

They can also be run directly:

```sh
gh-auto-switcher status
gh-auto-switcher doctor
```

`status` reports the working directory, account source, detected target host,
and credential mode. It never prints a token.

`doctor` checks the selected stored account by attempting a private
`gh auth token` lookup. With an explicit caller token, it reports that the
explicit token is present; it does not make an API request and does not prove
that a remote operation will succeed.

## Command and environment reference

| Input | Meaning |
| --- | --- |
| `github.account` | Effective Git config key used as the stored account login. |
| `GH_AUTO_SWITCHER_ACCOUNT` | Per-process account override. |
| `GH_AUTO_SWITCHER_REAL_GH` | Explicit real `gh` executable path. |
| `GH_TOKEN`, `GITHUB_TOKEN` | Explicit token family for `github.com` and `*.ghe.com`; never overwritten. |
| `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN` | Explicit token family for other Enterprise hosts; never overwritten. |
| `GIT_CONFIG_*` | Inherited by the Git subprocess, so Git's own scope and include behavior remains active. |
| `gh-auto-switcher exec -- ARGV` / `gh auto-switcher exec -- ARGV` | Internal forwarding modes used by the shell hooks. |

The target host is checked from recognized `--hostname`, `GH_HOST`, host-qualified
`GH_REPO`/`-R`, selected repository target arguments, and actual Git remotes.
For the selected command forms, `repo clone`/`repo view` accept a repository
reference, while `pr`/`issue`/`discussion` target arguments are checked as URL
forms. Host inference is not exhaustive for aliases, extensions, or arbitrary
payloads.

## Routing and safety

The decision order is intentionally conservative:

1. `auth`, `help`, `version`, `completion`, `config`, and `alias` control-plane
   commands use stock `gh` without account injection.
2. Conflicting host signals are rejected before credentials are considered.
3. A single applicable explicit caller token is passed through unchanged. For
   `github.com` and `*.ghe.com`, the applicable variables are `GH_TOKEN` and
   `GITHUB_TOKEN`; other Enterprise hosts use the Enterprise variables. This
   precedence can bypass an invalid `github.account`.
4. With no applicable explicit token, a valid configured account is used only
   for an unambiguous `github.com` target.
5. A missing account and no caller token preserve stock `gh` behavior. In this
   case the launcher may skip host resolution rather than inventing a target.
6. An unauthenticated configured account fails clearly. It never falls back to
   another stored account.

A single unsupported Enterprise target is passed through only when the
applicable explicit token already exists. Unsupported automatic routing is
rejected. Absolute calls such as `/opt/homebrew/bin/gh pr list`, scripts that
do not source the hook, and direct calls to another `gh` function bypass this
launcher.

## Troubleshooting

### `gh` is not using the selected account

Check the hook and resolved account:

```sh
type -a gh
gh auto-switcher status
git config --show-origin --get github.account
```

If `type -a gh` does not show the function, source the shell hook again. If
`github.account` is missing, stock `gh` behavior is expected.

### `github.account is empty` or `invalid GitHub account`

Fix or remove the effective setting. Inspect the normal Git result and its
origin:

```sh
git config --show-origin --get-all github.account
```

An applicable explicit caller token takes precedence over account validation
errors for that invocation, but correcting the configuration is still
recommended.

### `no GitHub authentication found for account`

Authenticate that exact login with stock `gh`, then retry:

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

The launcher intentionally does not try a different account.

### `unsupported target host` or `ambiguous GitHub host context`

Use one target host and make sure the current repository remote agrees with
it. The launcher refuses to guess when `--hostname`, `GH_HOST`, `GH_REPO`,
`-R`, a recognized target argument, and Git remotes disagree. An applicable
explicit token can pass through one unsupported host, but it cannot override a
conflict.

### `could not find the real gh executable`

Use an explicit path:

```sh
command -v gh
GH_AUTO_SWITCHER_REAL_GH="$(command -v gh)" gh pr list
```

Do not point `GH_AUTO_SWITCHER_REAL_GH` at `gh-auto-switcher` itself.

## Release and trust

Release tags use the `v*` pattern. The release workflow runs tests and all
supported platform builds before creating a draft release, then uploads binaries
and `checksums.txt` before publishing it. It accepts only human `admin` or
`maintain` collaborators as the original tag actor and rerun actor. Build jobs
have read-only repository permissions; the publishing job references the
`release` environment.

Repository administrators must also configure these protections in GitHub
settings:

- a `v*` tag ruleset that limits tag creation, updates, and deletion to release
  maintainers;
- a `release` environment with a required human reviewer, self-review currently
  allowed for the sole maintainer, and deployment tags restricted to `v*`.

The environment reviewer and tag ruleset protections are separate GitHub
settings; they were not configured or verified by this session. When a second
maintainer is available, disable self-review for separation of duties.

Workflow checks cannot protect against someone who is already authorized to
modify the workflow itself. Do not create or publish a release from an
unreviewed workflow change.

## What is not implemented

- automatic modification of shell startup files;
- general automatic account routing for Enterprise hosts;
- Git credential-helper integration;
- SSH identity selection;
- exhaustive target inference for every `gh` alias or extension.
