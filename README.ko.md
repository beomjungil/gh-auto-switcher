# gh-auto-switcher

현재 작업 컨텍스트의 Git 최종 설정을 기준으로 저장된 GitHub 계정을 선택한
뒤, 일반적인 [GitHub CLI (`gh`)](https://cli.github.com/) 명령을 실행합니다.

[English README](README.md) · [영문 사용법](docs/en/usage.md) ·
[상세 사용법](docs/ko/usage.md) · [Architecture / 아키텍처](docs/en/architecture.md)

## 하는 일

`gh-auto-switcher`는 launcher입니다. 현재 저장소의 유효한 Git
설정에 `github.account = work-github-username`가 있다면, `gh pr list` 같은 일반 명령을
실행할 때 해당 계정의 저장된 토큰을 조회하고 실제 `gh` 자식 프로세스에만
전달합니다.

전역으로 활성화된 GitHub 계정을 바꾸거나 `hosts.yml`을 수정하지 않습니다.
토큰 파일이나 두 번째 저장소-계정 매핑 데이터베이스도 만들지 않습니다.

## 기능

- Git의 유효한 `github.account` 설정에서 인증된 GitHub 계정을 선택합니다.
- 선택한 token을 자식 `gh` 프로세스에만 주입합니다.
- Bash, Zsh, Fish shell hook으로 일반 `gh` 명령을 감쌀 수 있습니다.
- GitHub CLI의 account store와 authentication 관리 명령은 stock `gh`에 위임합니다.

자동 계정 주입은 충돌이 없는 `github.com` 대상에 한정됩니다. 지원 범위 밖의
Enterprise host 하나만 명확하고 그 host에 맞는 caller token이 이미 있으면
그대로 통과시킬 수 있지만, 서로 다른 host 신호가 있으면 token 우선순위보다
먼저 거부합니다. 모든 `gh` alias, extension, API payload를 이해하려고 하지
않으며 host 추론은 보수적으로 동작합니다.

자동화에서 사용하기 전에 [routing과 안전성 상세](docs/ko/usage.md#라우팅과-안전성)를
확인하세요.

## 사전 요구사항

- 사용할 Git 설정 패턴을 지원하는 Git;
- 선택할 계정으로 인증된 GitHub CLI (`gh`);
- `gh`를 투명하게 감쌀 Bash, Zsh 또는 Fish;
- source 설치나 local 개발을 선택하는 경우에만 Cargo/Rust.

## 빠른 시작

prebuilt GitHub CLI extension 또는 source 설치 방식을 사용할 수 있습니다.

### 방식 A: GitHub CLI extension

GitHub에서 extension을 설치하면 GitHub CLI가 현재 platform에 맞는 prebuilt
release asset을 선택합니다.

```sh
gh extension install beomjungil/gh-auto-switcher
gh auto-switcher help
```

release workflow는 `v*` tag에서 Linux amd64, macOS amd64, macOS arm64
asset을 만들도록 구성되어 있습니다. 아직 release가 없다면 GitHub CLI가 source
extension으로 fallback하므로 Cargo/Rust가 필요합니다.

### 방식 B: standalone source 실행 파일

launcher를 `PATH`에 설치합니다.

```sh
cd /path/to/gh-auto-switcher
cargo install --path .
gh-auto-switcher help
```

일반적인 GitHub CLI 인증 절차로 계정을 인증합니다. 계정마다 필요한 만큼
로그인 절차를 반복합니다.

```sh
gh auth login --hostname github.com
gh auth status --hostname github.com
```

작업 방식에 맞는 Git 설정 scope에 계정 convention을 지정합니다.

```sh
git config --global github.account personal-github-username
```

현재 shell에 hook을 적용합니다.

```sh
# Bash, `gh extension install beomjungil/gh-auto-switcher` 이후
eval "$(gh auto-switcher shell-hook bash)"

# Zsh, `gh extension install beomjungil/gh-auto-switcher` 이후
eval "$(gh auto-switcher shell-hook zsh)"

# Fish, `gh extension install beomjungil/gh-auto-switcher` 이후
source (gh auto-switcher shell-hook fish | psub)

# 또는 방식 B standalone 실행 파일 사용:
# eval "$(gh-auto-switcher shell-hook bash)"
```

토큰을 출력하지 않고 계정 선택을 확인한 뒤, 평소처럼 명령을 사용합니다.

```sh
gh auto-switcher status
gh auto-switcher doctor
gh pr list
gh api repos/OWNER/REPOSITORY
```

conditional include, 명령별 override, shell hook 제거, 문제 해결은
[상세 사용법](docs/ko/usage.md)을 참고하세요.

## 문서 안내

| 필요한 내용 | English | 한국어 |
| --- | --- | --- |
| 시작하기 | [README](README.md) | 이 문서 |
| 설치와 사용 | [Usage](docs/en/usage.md) | [사용법](docs/ko/usage.md) |
| 설계와 보장 범위 | [Architecture](docs/en/architecture.md) | [아키텍처](docs/ko/architecture.md) |

## 보안 모델

- 선택한 토큰은 실제 `gh auth token` 명령으로 요청합니다.
- 토큰 출력은 비공개로 캡처하며 출력하거나 저장하지 않습니다.
- 토큰은 자식 `gh` 프로세스의 `GH_TOKEN`으로만 전달합니다.
- `gh auth login`, `logout`, `switch`, `refresh`, `token`, `status` 같은 인증
  관리 명령은 stock `gh` 동작을 사용합니다.
- 설정된 계정에서 토큰을 얻지 못하면 명확히 실패하며 다른 계정으로 조용히
  fallback하지 않습니다.

전체 흐름은
[아키텍처: credential lifecycle](docs/ko/architecture.md#credential-lifecycle)을
확인하세요.

## 개발

local extension 개발에서는 프로젝트 디렉터리에서 설치 명령을 실행합니다.
GitHub CLI가 해당 checkout을 가리키는 link로 관리합니다.

```sh
cd /path/to/gh-auto-switcher
gh extension install .
```

source 검사와 test:

```sh
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build --release
python3 -m pip install --user PyYAML==6.0.2
python3 tests/release_workflow_test.py
```

테스트는 실제 Git, 격리된 임시 설정, 가짜 `gh` 실행 파일, 실제 shell
프로세스, 격리된 local `gh extension install` 실행, prebuilt extension 실행
경로를 사용합니다. 실제 credential을 사용하지 않으며 사용자의 Git·shell·
GitHub CLI extension 설정을 수정하지 않습니다.
