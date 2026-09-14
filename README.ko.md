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
- Git
- 사용할 GitHub 계정의 `gh auth` 인증
- 일반 `gh ...` 명령을 투명하게 계정 전환할 때만 Bash, Zsh 또는 Fish

launcher 설치 방식은 하나를 선택합니다. 아래의 계정 인증, Git 설정, shell
hook 단계는 두 방식에 공통으로 적용됩니다.

### 방식 A: prebuilt GitHub CLI extension (권장)

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

현재 release는 Linux amd64, macOS amd64, macOS arm64 binary를 제공합니다.
GitHub CLI가 현재 platform에 맞는 binary를 선택합니다.

### 방식 B: standalone source 실행 파일

`gh-auto-switcher`를 `PATH`에서 직접 실행하려면 사용합니다. 이 방식은
Cargo/Rust가 필요합니다.

```sh
cd /path/to/gh-auto-switcher
cargo install --path .
gh-auto-switcher help
```

## 계정 인증

사용할 GitHub 계정을 일반적인 GitHub CLI 명령으로 각각 인증합니다. 계정
profile은 GitHub CLI가 관리하며, 이 extension은 profile을 변경하지 않습니다.
선택할 계정마다 login 절차를 반복합니다.

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

저장된 login이 기대한 GitHub username인지 확인하세요.

```sh
gh auth status --hostname github.com
```

## 계정 선택 설정

launcher는 각 명령마다 현재 working directory에서 Git의 최종 설정을 읽습니다.
다음 순서에서 먼저 적용되는 계정 규칙을 사용합니다.

1. `GH_AUTO_SWITCHER_ACCOUNT`: 하나의 process에서 사용할 계정
2. `github.account`: 명시적으로 지정한 계정 login
3. `user.name`: 유효한 GitHub username이고 동일한 `gh` 인증 profile이 있을 때
4. 일치하는 profile이 없으면 실제 `gh`를 변경 없이 실행

`personal-github-username`, `work-github-username` 같은 예시 값은
`gh auth status`에 표시되는 실제 GitHub login으로 바꿔 사용하세요.

최종 `user.name`이 인증된 GitHub profile의 username과 같으면
`github.account`를 별도로 설정할 필요가 없습니다.

```ini
[user]
    name = personal-github-username
```

conditional Git identity를 사용하면 `github.account` 설정 없이도 repository마다
서로 다른 인증 profile을 선택할 수 있습니다.

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

Git commit identity가 표시 이름이거나 GitHub login과 다르면 명시적인
`github.account` mapping을 사용합니다. 필요한 repository 범위에 맞춰 좁은
scope부터 선택하세요.

하나의 repository에만 적용하려면 local 설정을 사용합니다.

```sh
cd /path/to/customer-repository
git config --local github.account work-github-username
```

여러 repository 그룹에는 conditional include를 사용할 수 있습니다.

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

모든 repository에서 같은 계정을 명시적 기본값으로 사용할 때만 global 설정을
사용합니다.

```sh
git config --global github.account personal-github-username
```

`github.account`는 `user.name`보다 우선하는 명시 설정입니다. 따라서 repository마다
서로 다른 `user.name`으로 계정을 자동 선택하려면 global 값을 설정하지 마세요.
필요하면 local 또는 conditional `github.account`로 더 좁은 범위를 지정할 수
있습니다.

commit에는 표시 이름을 유지하고 GitHub 계정만 연결하려면 다음처럼 설정합니다.

```ini
[user]
    name = Beom Jungil

[github]
    account = personal-github-username
```

현재 context의 값과 출처를 확인합니다.

```sh
git config --get user.name
git config --get github.account
git config --show-origin --get user.name
git config --show-origin --get github.account
gh auth status --hostname github.com
```

유효하지 않은 표시 이름이거나 `user.name`과 일치하는 인증 profile이 없으면
다른 계정을 추측하지 않습니다. 일반적인 stock `gh` 동작을 유지합니다.
명시한 `github.account` 또는 `GH_AUTO_SWITCHER_ACCOUNT`를 인증할 수 없을 때는
다른 계정으로 조용히 fallback하지 않고 명확히 실패합니다.

## 일반 `gh` 명령에 shell hook 사용

현재 repository에 따라 `gh pr list` 같은 일반 명령이 계정을 자동 선택하게
하려면 shell hook을 사용합니다. GitHub CLI extension만으로는 `gh` 실행 파일을
대체할 수 없으며, `gh extension install`도 shell startup 파일을 수정하지
않습니다.

launcher를 먼저 설치한 뒤, 설치 방식에 맞는 hook 명령 하나만
`~/.bashrc`, `~/.zshrc`, 또는 Fish의 `~/.config/fish/config.fish`에 추가하세요.
startup 파일에 추가했다면 새 shell을 열고, 즉시 적용하려면 같은 명령을 현재
shell에서 실행하거나 source하세요.

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

설치 방식에 맞는 명령 하나만 사용하세요. hook이 활성화되면 평소처럼
사용합니다.

```sh
gh auto-switcher status
gh auto-switcher doctor
gh pr list
gh issue list
gh api repos/OWNER/REPOSITORY
```

hook은 얇은 shell function일 뿐입니다. parent shell에 token을 export하거나
디렉터리를 바꾸거나 GitHub CLI의 active account를 변경하지 않습니다.

## shell hook 없이 launcher 직접 호출

launcher를 명시적으로 호출하면 shell hook이 필요하지 않습니다.

```sh
gh auto-switcher exec -- pr list
gh auto-switcher exec -- api repos/OWNER/REPOSITORY
```

standalone binary를 `PATH`에 설치한 경우에는 다음처럼 실행합니다.

```sh
gh-auto-switcher exec -- pr list
gh-auto-switcher exec -- api repos/OWNER/REPOSITORY
```

hook도 `exec` 호출도 하지 않고 `gh pr list`를 실행하면 stock GitHub CLI가
실행되며 이 extension은 개입하지 않습니다.

repository 밖에서 실행하거나 일회성으로 계정을 지정할 때는 process override를
사용합니다.

```sh
GH_AUTO_SWITCHER_ACCOUNT=carter-hp gh auto-switcher exec -- pr list
```

## 선택 결과 확인

`status`는 token을 출력하지 않고 계정 출처, target host, credential mode를
보여줍니다. 설치 방식에 맞는 명령을 사용하세요.

```sh
# GitHub CLI extension
gh auto-switcher status

# Standalone executable
gh-auto-switcher status
```

`doctor`는 실제 GitHub CLI credential store에서 선택한 계정의 token을 가져올
수 있는지만 확인합니다. API 요청은 보내지 않습니다.

```sh
# GitHub CLI extension
gh auto-switcher doctor

# Standalone executable
gh-auto-switcher doctor
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
