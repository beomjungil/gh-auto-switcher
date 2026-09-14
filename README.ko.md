# gh-auto-switcher

현재 Git repository context에 맞는 GitHub 계정으로 GitHub CLI(`gh`)를
실행합니다.

`gh`는 같은 GitHub host에 여러 계정을 저장할 수 있지만, 일반적인 계정
선택은 host별 전역 active account를 따릅니다. 이 extension은 계정 선택을
명령 단위로 처리합니다. Git이 현재 identity를 결정하고, launcher는 그
계정의 token만 실제 `gh` 자식 프로세스에 전달합니다.

## 설치

필요한 것:

- GitHub CLI(`gh`)
- 사용할 GitHub 계정의 `gh auth` 인증
- 일반 `gh ...` 명령을 투명하게 계정 전환할 때만 Bash, Zsh 또는 Fish

prebuilt extension을 설치합니다.

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

현재 release는 Linux amd64, macOS amd64, macOS arm64 binary를 제공합니다.
GitHub CLI가 현재 platform에 맞는 binary를 선택합니다.

## 계정 인증

사용할 GitHub 계정을 일반적인 GitHub CLI 명령으로 각각 인증합니다. 계정
profile은 GitHub CLI가 관리하며, 이 extension은 profile을 변경하지 않습니다.

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

계정을 추가할 때 login 절차를 다시 실행합니다. 기대한 GitHub username이
저장되었는지 확인하세요.

```sh
gh auth status --hostname github.com
```

## 계정 선택 규칙

launcher는 각 명령마다 현재 working directory에서 Git의 최종 설정을 읽습니다.
다음 순서에서 먼저 적용되는 규칙을 사용합니다.

1. `GH_AUTO_SWITCHER_ACCOUNT`: 해당 process에서 사용할 계정
2. `github.account`: 명시적으로 지정한 계정
3. `user.name`: 유효한 GitHub username이고 동일한 `gh` 인증 profile이 있을 때
4. 일치하는 profile이 없으면 실제 `gh`를 변경 없이 실행

`github.account`는 선택 사항입니다. Git commit identity와 GitHub login이
다를 때 사용합니다.

Git identity 자체가 GitHub username이면 추가 설정이 필요하지 않습니다.

```ini
[user]
    name = beomjungil
```

표시용 Git identity가 GitHub username과 다르면 명시적으로 지정합니다.

```ini
[user]
    name = Beom Jungil

[github]
    account = beomjungil
```

Git의 local/global 설정과 `includeIf`가 그대로 적용됩니다. repository별로
계정을 선택할 수도 있습니다.

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

현재 context의 값과 출처를 확인합니다.

```sh
git config --get user.name
git config --get github.account
git config --show-origin --get user.name
gh auth status --hostname github.com
```

유효하지 않은 표시 이름이거나 `user.name`과 일치하는 인증 profile이 없으면
다른 계정을 추측하지 않습니다. 일반적인 stock `gh` 동작을 유지합니다.
명시한 `github.account` 또는 `GH_AUTO_SWITCHER_ACCOUNT`를 인증할 수 없을 때는
다른 계정으로 조용히 fallback하지 않고 명확히 실패합니다.

## 실행 방식 선택

### 방식 A: 일반 `gh` 명령을 투명하게 사용

현재 repository에 따라 `gh pr list` 같은 명령이 계정을 자동 선택하게 하려면
이 방식을 사용합니다. GitHub CLI extension만으로는 `gh` 실행 파일을 대체할
수 없으므로 shell hook이 필요합니다.

hook은 shell startup 파일을 자동으로 수정하지 않습니다. 아래 명령을 startup
파일에 추가하거나 현재 shell에서만 실행하세요.

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

hook이 활성화되면 평소처럼 사용합니다.

```sh
gh auto-switcher status
gh auto-switcher doctor
gh pr list
gh issue list
gh api repos/OWNER/REPOSITORY
```

hook은 얇은 shell function일 뿐입니다. parent shell에 token을 export하거나
디렉터리를 바꾸거나 GitHub CLI의 active account를 변경하지 않습니다.

### 방식 B: shell hook 없이 launcher를 직접 호출

launcher를 명시적으로 호출하면 shell hook이 필요하지 않습니다.

```sh
gh auto-switcher exec -- pr list
gh auto-switcher exec -- api repos/OWNER/REPOSITORY
```

standalone binary를 `PATH`에 설치한 경우에는 다음처럼 실행합니다.

```sh
gh-auto-switcher exec -- pr list
```

hook도 `exec` 호출도 하지 않고 `gh pr list`를 실행하면 stock GitHub CLI가
실행되며 이 extension은 개입하지 않습니다.

repository 밖에서 실행하거나 일회성으로 계정을 지정할 때는 process override를
사용합니다.

```sh
GH_AUTO_SWITCHER_ACCOUNT=carter-hp gh auto-switcher exec -- pr list
```

## 선택 결과 확인

`status`는 token을 출력하지 않고 계정 출처, target host, credential mode을
보여줍니다.

```sh
gh auto-switcher status
```

`doctor`는 실제 GitHub CLI credential store에서 선택한 계정의 token을 가져올
수 있는지만 확인합니다. API 요청은 보내지 않습니다.

```sh
gh auto-switcher doctor
```

## launcher의 동작 범위

- 선택한 token은 자식 `gh` process에만 전달합니다.
- GitHub CLI의 전역 active account를 바꾸지 않습니다.
- `hosts.yml`을 수정하거나 token을 저장하거나 별도 계정 mapping database를
  만들지 않습니다.
- 적용 가능한 `GH_TOKEN`/`GITHUB_TOKEN`이 있으면 그대로 보존하고 설정된 계정보다
  우선합니다.
- `gh auth login`, `logout`, `switch`, `refresh`, `token`, `status` 같은 인증
  관리 명령은 실제 `gh` 동작을 사용합니다.
- 자동 token 주입은 명확한 `github.com` target에 한정합니다. host 신호가
  충돌하면 추측하지 않고 실패합니다.

`/opt/homebrew/bin/gh pr list` 같은 절대 경로 호출, hook을 source하지 않은
script, launcher를 우회하는 다른 shell function은 가로채지 않습니다.

## 문제 해결

상세한 routing, shell, conditional include, host, 실패 동작은
[사용법 문서](docs/ko/usage.md)를 참고하세요. 보안 경계는
[아키텍처 문서](docs/ko/architecture.md)에 설명되어 있습니다.

유용한 확인 명령:

```sh
type -a gh
gh auto-switcher status
git config --show-origin --get user.name
git config --show-origin --get github.account
gh auth status --hostname github.com
```

`type -a gh`에서 hook function이 실제 `gh`보다 먼저 나오지 않으면 현재
shell에서 hook을 다시 실행합니다. 일치하는 `user.name` profile이 없다면
stock `gh` 동작이 되는 것이 정상입니다.

## Contribute

repository를 clone하고 local 검사를 실행합니다.

```sh
git clone https://github.com/beomjungil/gh-auto-switcher.git
cd gh-auto-switcher
cargo fmt --check
cargo test --locked
cargo clippy --all-targets -- -D warnings
cargo build --release --locked
python3 tests/release_workflow_test.py
```

local extension entrypoint를 확인하려면:

```sh
gh extension install .
gh auto-switcher help
```

테스트는 격리된 Git 및 GitHub CLI fixture를 사용하며 개인 Git 설정, credential,
shell startup 파일, 설치된 extension 설정을 변경하지 않습니다.
