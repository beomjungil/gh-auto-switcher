# 사용법

이 문서는 `gh-auto-switcher`의 실제 사용 절차를 설명하는 how-to 안내서입니다.
왜 이런 동작을 하는지 알고 싶다면
[아키텍처 문서](architecture.md)를 참고하세요.

[English usage guide](../en/usage.md) · [English README](../../README.md) ·
[한국어 README](../../README.ko.md)

## 1. launcher 설치

`gh-auto-switcher`는 prebuilt GitHub CLI extension과 source 설치 방식을
지원합니다. shell startup 파일은 자동으로 수정하지 않습니다.

### GitHub CLI remote extension

배포 저장소에서 extension을 설치합니다.

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

matching release asset이 있으면 GitHub CLI가 해당 prebuilt binary를 설치·
실행합니다. release workflow는 Linux amd64, macOS amd64, macOS arm64 asset을
만들도록 구성되어 있습니다. prebuilt 설치에는 Cargo/Rust가 필요하지 않습니다.

첫 release 전에는 GitHub CLI가 repository root의 `gh-auto-switcher` source
entrypoint로 fallback합니다. 이 entrypoint는 매 invocation마다 다음 build를
수행합니다. release는 있지만 현재 platform용 asset이 없으면 source로 fallback하지
않고 설치가 실패합니다.

```sh
cargo build --release --locked --manifest-path /path/to/gh-auto-switcher/Cargo.toml
```

이후 build artifact를 원래 working directory와 argument를 유지한 채 실행합니다.
source extension을 실행하려면 실행 환경에 Cargo/Rust가 필요합니다.

### GitHub CLI local extension (개발용)

프로젝트 디렉터리는 GitHub CLI local extension convention
(`gh-<extension-name>`)에 맞고 root 실행 파일도 같은 이름을 갖습니다. local
개발에서만 checkout으로 설치합니다.

```sh
cd /path/to/gh-auto-switcher
gh extension install .
```

local extension은 checkout을 가리키는 link로 관리되며 설치 과정에서 project를
build하지 않습니다.

### Standalone source 실행 파일

프로젝트 디렉터리에서 launcher를 `PATH`에 설치합니다.

```sh
cd /path/to/gh-auto-switcher
cargo install --path .
```

설치된 실행 파일을 확인합니다.

```sh
command -v gh-auto-switcher
gh-auto-switcher help
```

현재 소스 빌드를 검증한 toolchain은 Rust 1.93.1입니다. 그보다 낮은 최소
지원 Rust 버전은 아직 보장하지 않습니다.

Cargo로 설치한 binary를 제거하려면 다음을 실행합니다.

```sh
cargo uninstall gh-auto-switcher
```

GitHub CLI extension을 제거하려면 다음을 실행합니다.

```sh
gh extension remove auto-switcher
```

extension을 제거해도 source checkout이나 별도로 설치한 standalone binary는
삭제되지 않습니다.

실제 GitHub CLI가 `PATH`에서 검색되지 않는다면 해당 프로세스나 shell
세션에 경로를 지정할 수 있습니다.

```sh
GH_AUTO_SWITCHER_REAL_GH=/opt/homebrew/bin/gh gh-auto-switcher status
export GH_AUTO_SWITCHER_REAL_GH=/opt/homebrew/bin/gh
```

launcher는 지정 경로가 실행 권한이 있는 파일인지 확인하며, launcher 자신을
가리키는 경로는 거부합니다.

### extension 업데이트

remote extension은 GitHub CLI로 업데이트합니다.

```sh
gh extension upgrade auto-switcher
```

local source extension은 checkout을 업데이트한 뒤 extension 명령을 다시
실행합니다.

```sh
cd /path/to/gh-auto-switcher
git pull
gh auto-switcher help
```

local source entrypoint가 실행 전에 release artifact를 다시 빌드합니다.

## 2. stock `gh`로 계정 인증

일반적인 GitHub CLI 인증 절차로 계정을 인증합니다. 이 프로젝트는 GitHub
CLI credential store를 관리하지 않습니다.

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

선택하려는 각 계정에 대해 필요한 만큼 로그인 절차를 반복합니다. 이
프로젝트에서 사용하는 account 이름은 `gh auth token`이 받아들이는 login이어야
합니다.

launcher는 선택한 계정의 token을 다음과 같은 명령으로 조회합니다.

```sh
gh auth token --hostname github.com --user ACCOUNT
```

다음 단계에서 설치하는 shell function이 아니라 실제 `gh`를 사용해 token을
조회합니다.

## 3. shell hook 설치

hook은 얇은 `gh` function을 정의합니다. 일반 인자를 launcher로 전달하며,
`cd`를 가로채거나 token을 export하거나 parent shell의 working directory를
변경하지 않습니다.

binary 설치 후 shell startup 파일에 다음 중 하나를 추가하거나, 현재 shell에서
일시적으로 실행합니다.

### Bash

```sh
# `gh extension install beomjungil/gh-auto-switcher` 이후
eval "$(gh auto-switcher shell-hook bash)"

# standalone `cargo install --path .` 이후
eval "$(gh-auto-switcher shell-hook bash)"
```

### Zsh

```sh
# `gh extension install beomjungil/gh-auto-switcher` 이후
eval "$(gh auto-switcher shell-hook zsh)"

# standalone `cargo install --path .` 이후
eval "$(gh-auto-switcher shell-hook zsh)"
```

### Fish

```fish
# `gh extension install beomjungil/gh-auto-switcher` 이후
source (gh auto-switcher shell-hook fish | psub)

# standalone `cargo install --path .` 이후
source (gh-auto-switcher shell-hook fish | psub)
```

생성된 shell hook은 먼저 `PATH`에 standalone `gh-auto-switcher`가 있는지
확인합니다. 있으면 해당 실행 파일을 호출하고, 없으면 shell function을
우회하기 위해 `command gh auto-switcher exec -- ...`로 설치된 extension에
다시 진입합니다. 따라서 같은 hook을 prebuilt extension과 standalone 설치에서
사용할 수 있습니다. launcher는 매번 실행될 때 `PATH`에서 자기 자신을 제외한
실제 `gh`를 찾으며 경로를 캡처하거나 cache하지 않습니다. 이 검색이 적절하지
않다면 `GH_AUTO_SWITCHER_REAL_GH`를 사용하세요.

hook이 활성화되었는지 확인합니다.

```sh
type -a gh
```

정확한 출력은 shell마다 다르지만 function이 실제 `gh` 실행 파일보다 먼저
표시되어야 합니다.

### hook 제거

startup 파일에서 `eval` 또는 `source` 줄을 지우고 새 shell을 시작합니다.
현재 shell에서 즉시 제거하려면 해당 shell의 명령을 사용합니다.

```sh
# Bash
unset -f gh

# Zsh
unfunction gh
```

```fish
functions --erase gh
```

다른 도구가 먼저 설치한 `gh` function이 있었다면 무조건 지우지 말고 원래
function을 저장하거나 복원하세요.

## 4. account 선택 설정

launcher는 현재 working directory에서 Git의 최종 설정을 읽습니다. 선택 순서는
다음과 같습니다.

1. `GH_AUTO_SWITCHER_ACCOUNT`: 하나의 process에서 사용할 account
2. `github.account`: 명시적으로 지정한 account login
3. `user.name`: 유효한 GitHub login이고 동일한 `gh` 인증 profile이 있을 때
4. 일치하는 profile이 없으면 stock `gh`

현재 값과 출처를 확인합니다.

```sh
git config --get user.name
git config --get github.account
git config --show-origin --get user.name
git config --show-origin --get github.account
```

`user.name`이 로그인된 GitHub username과 같으면 추가 설정이 필요하지 않습니다.

```ini
[user]
    name = beomjungil
```

Git identity가 표시 이름 등으로 GitHub login과 다르면 `github.account`를
추가합니다.

```ini
[user]
    name = Beom Jungil

[github]
    account = beomjungil
```

명시적인 account 값은 ASCII 문자·숫자·하이픈으로 이루어진 1–39자여야
합니다. 잘못된 명시 설정이나 해당 account의 token이 없으면 다른 account로
fallback하지 않고 실패합니다. 반면 `github.account`가 없는 `user.name`은
유효한 login이 아니거나 일치하는 profile이 없을 때 stock `gh`를 사용합니다.

### conditional include

Git이 설정을 평가합니다. launcher가 Git의 condition 문법을 다시 구현하지
않으므로 일반적인 include와 우선순위 동작이 유지됩니다.

예를 들어 remote URL에 따라 Git identity와 matching GitHub profile을 바꿀 수
있습니다.

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

설치된 Git 버전이 지원하는 `gitdir`, `gitdir/i`, `onbranch`, 중첩 include,
linked worktree, local/global 우선순위도 Git이 직접 평가합니다. integration
suite는 실제 Git으로 이 패턴들을 검증합니다.

## 5. 일반 명령 실행

launcher를 사용하는 방법은 두 가지입니다.

### 일반 `gh` 명령을 투명하게 사용

`gh pr list`처럼 평소 명령을 자동 전환하려면 shell hook이 필요합니다.
GitHub CLI extension만으로는 `gh` 실행 파일을 대체할 수 없습니다. hook은
shell startup 파일을 자동으로 수정하지 않습니다.

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

hook이 활성화되면 다음처럼 사용합니다.

```sh
gh pr list
gh api repos/OWNER/REPOSITORY
gh issue list --limit 10
gh repo view OWNER/REPOSITORY
```

실행할 때마다 현재 working context의 Git 설정을 다시 평가합니다. hook이
없으면 일반 `gh`는 stock GitHub CLI이며 launcher가 개입하지 않습니다.

### shell hook 없이 launcher 직접 호출

launcher를 직접 호출하면 shell hook이 필요하지 않습니다.

```sh
gh auto-switcher exec -- pr list
gh auto-switcher exec -- api repos/OWNER/REPOSITORY
```

standalone binary를 `PATH`에 설치했다면 다음처럼 실행합니다.

```sh
gh-auto-switcher exec -- pr list
```

repository 밖에서 실행하거나 일회성으로 account를 지정할 때는 process
override를 사용합니다.

```sh
GH_AUTO_SWITCHER_ACCOUNT=carter-hp gh auto-switcher exec -- pr list
```

이 override는 환경 변수이므로 launcher나 Git 설정에 저장되지 않습니다.

## 관리 명령

shell function을 통해 실행할 수 있습니다.

```sh
gh auto-switcher status
gh auto-switcher doctor
gh auto-switcher shell-hook bash
```

직접 실행할 수도 있습니다.

```sh
gh-auto-switcher status
gh-auto-switcher doctor
```

`status`는 working directory, account source, 감지한 target host, credential
mode을 보여줍니다. token은 출력하지 않습니다.

`doctor`는 비공개 `gh auth token` 조회를 시도해 선택된 저장 account를
확인합니다. caller token이 명시된 경우 명시적 token이 있다는 사실을
보고합니다. API 요청은 하지 않으므로 remote 작업의 성공까지 보장하지는
않습니다.

## 명령 및 환경 변수 reference

| 입력 | 의미 |
| --- | --- |
| `github.account` | 선택 사항인 Git 최종 설정 key. 지정하면 저장된 account login을 명시적으로 선택 |
| `user.name` | `github.account`가 없을 때 matching `gh` profile이 있으면 사용하는 Git identity |
| `GH_AUTO_SWITCHER_ACCOUNT` | 가장 높은 우선순위의 프로세스별 account override |
| `GH_AUTO_SWITCHER_REAL_GH` | 실제 `gh` 실행 파일의 명시적 경로 |
| `GH_TOKEN`, `GITHUB_TOKEN` | `github.com` 및 `*.ghe.com`용 명시적 token family. 덮어쓰지 않음 |
| `GH_ENTERPRISE_TOKEN`, `GITHUB_ENTERPRISE_TOKEN` | 그 밖의 Enterprise host용 명시적 token family. 덮어쓰지 않음 |
| `GIT_CONFIG_*` | Git subprocess에 상속되므로 Git의 scope/include 동작 유지 |
| `gh-auto-switcher exec -- ARGV` / `gh auto-switcher exec -- ARGV` | shell hook이 사용하는 내부 forwarding mode |

target host는 인식된 `--hostname`, `GH_HOST`, host-qualified `GH_REPO`/`-R`,
선별된 repository target argument, 실제 Git remote에서 확인합니다. 인식된
명령 중 `repo clone`/`repo view`의 repository target은 repository reference로,
`pr`/`issue`/`discussion` target은 URL 형식으로 검사합니다. alias, extension,
임의 payload까지 host를 전부 추론하지는 않습니다.

## 라우팅과 안전성

결정 순서는 보수적으로 설계되어 있습니다.

1. `auth`, `help`, `version`, `completion`, `config`, `alias` control-plane
   명령은 account token을 주입하지 않고 stock `gh`를 사용합니다.
2. 서로 다른 host 신호가 있으면 credential을 검토하기 전에 실패합니다.
3. 하나의 target에 적용되는 명시적 caller token이 있으면 그대로 통과시킵니다.
   `github.com` 및 `*.ghe.com`은 `GH_TOKEN`/`GITHUB_TOKEN`, 그 밖의 Enterprise
   host는 Enterprise 변수를 사용합니다. 이 token은 account validation error보다
   우선합니다.
4. 적용 가능한 명시적 token이 없으면 `github.account`가 있을 때 그 account를
   사용합니다. 없으면 유효한 `user.name`과 일치하는 인증 profile이 있을 때만
   해당 account를 충돌 없는 `github.com` target에 사용합니다.
5. 일치하는 `user.name` profile이 없으면 stock `gh` 동작을 유지합니다. 이
   경우 launcher는 target을 임의로 만들지 않고 host 확인을 생략할 수 있습니다.
6. 명시한 account에서 token을 얻지 못하면 명확히 실패하며 다른 저장 account로
   fallback하지 않습니다. `user.name` profile이 없는 경우만 의도적으로 stock
   `gh`로 처리합니다.

지원되지 않는 Enterprise target 하나는 해당 host에 적용되는 explicit token이
이미 있을 때만 그대로 통과합니다. 자동 account routing은 거부됩니다.
`/opt/homebrew/bin/gh pr list` 같은 절대 경로 호출, hook을 source하지 않은
script, 다른 `gh` function을 통한 호출은 의도적으로 이 launcher를 우회합니다.

## 문제 해결

### 선택한 account가 사용되지 않음

hook과 account를 확인합니다.

```sh
type -a gh
gh auto-switcher status
git config --show-origin --get github.account
```

`type -a gh`에 function이 없으면 shell hook을 다시 source하세요. 명시적인
`gh auto-switcher exec -- ...` 호출에는 shell hook이 필요하지 않습니다.
`github.account`가 없고 일치하는 `user.name` profile도 없으면 stock `gh` 동작이
정상입니다.

### `github.account is empty` 또는 `invalid GitHub account`

최종 명시 설정을 수정하거나 제거합니다. 일반 Git 결과와 origin을 확인합니다.

```sh
git config --show-origin --get-all github.account
```

`github.account`가 없으면 launcher는 최종 `user.name`이 유효한 GitHub login인지
확인하고 동일한 인증 profile이 있을 때만 사용합니다. 그렇지 않으면 stock `gh`를
유지합니다.

해당 invocation에 적용되는 explicit caller token은 account validation error보다
우선할 수 있지만, 설정 자체를 수정하는 것을 권장합니다.

### `no GitHub authentication found for account`

명시적인 `github.account` 또는 `GH_AUTO_SWITCHER_ACCOUNT`를 사용했다면 정확히
그 login을 stock `gh`로 인증한 뒤 다시 시도합니다.

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

launcher는 다른 account를 대신 시도하지 않습니다. `user.name`에서 자동으로
선택한 profile이 token을 제공하지 못하는 경우에도 인증 오류를 명확히 보여줍니다.
profile 자체가 없으면 stock `gh` 동작으로 처리됩니다.

### `unsupported target host` 또는 `ambiguous GitHub host context`

하나의 target host만 사용하고 현재 repository remote와 일치하는지 확인합니다.
`--hostname`, `GH_HOST`, `GH_REPO`, `-R`, 인식된 target argument, Git remote가
서로 다르면 launcher는 추측하지 않습니다. 적용 가능한 explicit token은 단일
unsupported host를 통과시킬 수 있지만 충돌을 무시할 수는 없습니다.

### `could not find the real gh executable`

명시적 경로를 사용합니다.

```sh
command -v gh
GH_AUTO_SWITCHER_REAL_GH="$(command -v gh)" gh pr list
```

`GH_AUTO_SWITCHER_REAL_GH`를 `gh-auto-switcher` 자신으로 지정하지 마세요.

## Release와 trust

release tag는 `v*` pattern을 사용합니다. release workflow는 draft release를
만들기 전에 test와 지원 platform build를 모두 통과해야 하며, 이후 binary와
`checksums.txt`를 upload한 뒤 publish합니다. 최초 tag actor와 rerun actor가
human `admin` 또는 `maintain` collaborator인지 확인합니다. build job은
read-only repository permission만 가지며 publish job이 `release` environment를
참조합니다.

repository administrator는 GitHub 설정에서도 다음 보호를 구성해야 합니다.

- tag 생성·수정·삭제를 release maintainer로 제한하는 `v*` tag ruleset;
- human reviewer가 필요하고, 현재는 sole maintainer를 위해 self-review를
  허용하며, deployment tag를 `v*`로 제한한 `release` environment.

environment reviewer와 tag ruleset 보호는 별도의 GitHub 설정이며, 이번 session에서
구성·검증하지 않았습니다. 두 번째 maintainer가 생기면 separation of duties를 위해
self-review를 끄세요.

workflow 자체를 수정할 권한이 있는 사용자를 workflow check만으로 막을 수는
없습니다. review되지 않은 workflow 변경으로 release를 만들거나 publish하지
마세요.

## 구현하지 않은 범위

- shell startup 파일 자동 수정;
- Enterprise host의 일반적인 자동 account routing;
- Git credential-helper 연동;
- SSH identity 선택;
- 모든 `gh` alias와 extension에 대한 완전한 target 추론.
