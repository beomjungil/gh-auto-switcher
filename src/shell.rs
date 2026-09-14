#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "bash" => Some(Self::Bash),
            "zsh" => Some(Self::Zsh),
            "fish" => Some(Self::Fish),
            _ => None,
        }
    }

    pub fn detect() -> Self {
        std::env::var("SHELL")
            .ok()
            .and_then(|shell| shell.rsplit('/').next().and_then(Self::parse))
            .unwrap_or(Self::Bash)
    }
}

pub fn hook(shell: Shell) -> &'static str {
    match shell {
        Shell::Bash | Shell::Zsh => {
            r#"gh() {
    if command -v gh-auto-switcher >/dev/null 2>&1; then
        command gh-auto-switcher exec -- "$@"
    else
        command gh auto-switcher exec -- "$@"
    fi
}
"#
        }
        Shell::Fish => {
            r#"function gh
    if command -q gh-auto-switcher
        command gh-auto-switcher exec -- $argv
    else
        command gh auto-switcher exec -- $argv
    end
end
"#
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{hook, Shell};

    #[test]
    fn emits_sourceable_hooks_for_every_supported_shell() {
        assert!(hook(Shell::Bash).contains("\"$@\""));
        assert!(hook(Shell::Zsh).contains("\"$@\""));
        assert!(hook(Shell::Fish).contains("$argv"));
    }
}
