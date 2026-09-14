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

## 4. Configure account selection

The launcher reads Git's effective configuration in the current working
directory. The selection order is:

1. `GH_AUTO_SWITCHER_ACCOUNT` for one process;
2. `github.account` as an explicit account login;
3. `user.name` when it is a valid GitHub login with a matching authenticated
   `gh` profile;
4. stock `gh` when no matching profile exists.

Inspect the values and their origins:

```sh
git config --get user.name
git config --get github.account
git config --show-origin --get user.name
git config --show-origin --get github.account
```

When `user.name` already equals a logged-in GitHub username, no extra
configuration is needed:

```ini
[user]
    name = beomjungil
```

Use `github.account` when the Git identity is a display name or otherwise
differs from the GitHub login:

```ini
[user]
    name = Beom Jungil

[github]
    account = beomjungil
```

The explicit account value must be 1–39 ASCII letters, numbers, or hyphens.
Empty values, slashes, spaces, and other malformed explicit values fail
instead of selecting a different account. An explicit account without an
available token also fails clearly. The implicit `user.name` path falls back
to stock `gh` when the value is not a valid login or no matching profile
exists.

### Conditional includes

Git evaluates the configuration. The launcher does not reimplement Git's
condition syntax, so normal include and precedence behavior is preserved.

For example, a remote URL condition can choose a different Git identity and
therefore a different matching GitHub profile:

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

Other Git-supported conditions, including `gitdir`, `gitdir/i`, `onbranch`,
nested includes, linked worktrees, and normal local/global precedence are
resolved by the installed Git version. The integration suite exercises these
patterns with real Git.

## 5. Run commands

There are two ways to use the launcher. Choose the one that matches how you
want to invoke `gh`.

### Transparent normal `gh` commands

A shell hook is required when you want ordinary commands such as `gh pr list`
to go through the launcher. The GitHub CLI extension cannot replace the `gh`
executable by itself. The hook does not edit shell startup files automatically.

For Bash, add this to the shell startup file or evaluate it for the current
shell:

```sh
eval "$(gh auto-switcher shell-hook bash)"
```

For Zsh, use:

```sh
eval "$(gh auto-switcher shell-hook zsh)"
```

For Fish:

```fish
source (gh auto-switcher shell-hook fish | psub)
```

Then ordinary commands use the selected account:

```sh
gh pr list
gh api repos/OWNER/REPOSITORY
gh issue list --limit 10
gh repo view OWNER/REPOSITORY
```

Each invocation re-evaluates Git configuration from the current working
context. A branch or remote change is considered by the next command without a
`cd` hook. Without the hook, a normal `gh pr list` is stock `gh` and this
launcher is not involved.

### Explicit launcher commands

A shell hook is not needed when the launcher is invoked explicitly:

```sh
gh auto-switcher exec -- pr list
gh auto-switcher exec -- api repos/OWNER/REPOSITORY
```

With a standalone binary on `PATH`, use:

```sh
gh-auto-switcher exec -- pr list
```

For a command outside a repository or targeting another repository, select an
account explicitly for that process:

```sh
GH_AUTO_SWITCHER_ACCOUNT=work-github-username gh auto-switcher exec -- repo clone customer/example
GH_AUTO_SWITCHER_ACCOUNT=personal-github-username gh auto-switcher exec -- --repo personal-github-username/example pr list
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
| `github.account` | Optional effective Git config key that explicitly selects the stored account login. |
| `user.name` | Fallback Git identity; used when it is a valid GitHub login with a matching authenticated profile. |
| `GH_AUTO_SWITCHER_ACCOUNT` | Per-process account override with highest account-selection precedence. |
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
4. With no applicable explicit token, `github.account` is used when present.
   Otherwise, `user.name` is used only when it is a valid GitHub login with a
   matching authenticated profile, and only for an unambiguous `github.com`
   target.
5. A missing or unmatched `user.name` and no caller token preserve stock `gh`
   behavior. In this case the launcher may skip host resolution rather than
   inventing a target.
6. An explicit account that cannot provide a token fails clearly. It never falls
   back to another stored account. An implicit `user.name` candidate with no
   matching profile is the exception and intentionally uses stock `gh`.

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

If `type -a gh` does not show the function, source the shell hook again. If you
are using an explicit `gh auto-switcher exec -- ...` command, no shell hook is
needed. A missing or unmatched `github.account` and `user.name` uses stock `gh`
behavior.

### `github.account is empty` or `invalid GitHub account`

Fix or remove the effective explicit setting. Inspect the normal Git result and
its origin:

```sh
git config --show-origin --get-all github.account
```

An applicable explicit caller token takes precedence over account validation
errors for that invocation, but correcting the configuration is still
recommended. Without `github.account`, the launcher checks whether the
configured `user.name` is a valid login with a matching authenticated profile;
otherwise it preserves stock `gh` behavior.

### `no GitHub authentication found for account`

For an explicit `github.account` or `GH_AUTO_SWITCHER_ACCOUNT`, authenticate
that exact login with stock `gh`, then retry:

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

The launcher intentionally does not try a different account. If the account
came only from `user.name`, a missing matching profile normally results in
stock `gh`; a profile that exists but cannot provide its token is reported as
an authentication error.

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
