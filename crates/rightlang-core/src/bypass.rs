//! Smart context auto-bypass detection for URLs, emails, file paths, code identifiers, and CLI flags.

/// Utility for detecting whether a given token/buffer should be bypassed
/// without attempting keyboard layout conversion.
pub struct SmartBypass;

impl SmartBypass {
    /// Determines whether the input string matches programming syntax,
    /// URLs, file paths, emails, or command-line flags.
    pub fn should_bypass(input: &str) -> bool {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return false;
        }

        // 1. Web URLs, schemes, and domains
        if trimmed.starts_with("http://")
            || trimmed.starts_with("https://")
            || trimmed.starts_with("ftp://")
            || trimmed.starts_with("ssh://")
            || trimmed.starts_with("git@")
            || trimmed.starts_with("www.")
        {
            return true;
        }

        // Domain-like ending (e.g. .com, .org, .net, .io, .co.th, .dev, .ai, .app)
        let domain_suffixes = [
            ".com", ".org", ".net", ".io", ".co.th", ".dev", ".ai", ".app", ".xyz", ".me", ".info",
        ];
        if domain_suffixes.iter().any(|s| trimmed.ends_with(s)) && trimmed.contains('.') {
            return true;
        }

        // 2. Email pattern (must contain @ with text before and domain after)
        if let Some(at_idx) = trimmed.find('@') {
            if at_idx > 0 && at_idx + 1 < trimmed.len() {
                let after_at = &trimmed[at_idx + 1..];
                if after_at.contains('.') && !after_at.starts_with('.') && !after_at.ends_with('.') {
                    return true;
                }
            }
        }

        // 3. File paths (Unix / Windows / Home)
        if trimmed.starts_with('/')
            || trimmed.starts_with("./")
            || trimmed.starts_with("../")
            || trimmed.starts_with("~/")
            || (trimmed.len() >= 3 && trimmed.chars().next().map(|c| c.is_ascii_alphabetic()).unwrap_or(false) && &trimmed[1..3] == ":\\")
        {
            return true;
        }

        // Common code / file extensions
        let file_extensions = [
            ".rs", ".py", ".js", ".ts", ".jsx", ".tsx", ".html", ".css", ".json",
            ".toml", ".yaml", ".yml", ".sh", ".bash", ".zsh", ".cpp", ".c", ".h",
            ".hpp", ".go", ".java", ".md", ".txt", ".sql", ".swift", ".kt",
        ];
        if file_extensions.iter().any(|ext| trimmed.ends_with(ext)) && trimmed.contains('.') {
            return true;
        }

        // 4. CLI flags (e.g. --help, --version, -rf, -p)
        if trimmed.starts_with("--") && trimmed.len() > 2 {
            let rest = &trimmed[2..];
            if rest.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
                return true;
            }
        } else if trimmed.starts_with('-') && trimmed.len() > 1 {
            let rest = &trimmed[1..];
            if rest.chars().all(|c| c.is_ascii_alphabetic()) {
                return true;
            }
        }

        // 5. Code identifiers:
        // C++ / Rust namespace separator `::`
        if trimmed.contains("::") {
            return true;
        }

        // Hexadecimal literals: 0x...
        if (trimmed.starts_with("0x") || trimmed.starts_with("0X")) && trimmed.len() > 2 {
            if trimmed[2..].chars().all(|c| c.is_ascii_hexdigit()) {
                return true;
            }
        }

        // Semantic versioning: v1.0.0 or 1.2.3
        if trimmed.starts_with('v') || trimmed.starts_with('V') {
            let rest = &trimmed[1..];
            if rest.contains('.') && rest.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return true;
            }
        }

        // snake_case or UPPER_SNAKE_CASE (contains '_' with alphanumeric on both sides)
        if trimmed.contains('_') {
            let parts: Vec<&str> = trimmed.split('_').collect();
            if parts.len() >= 2 && parts.iter().all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_alphanumeric())) {
                return true;
            }
        }

        false
    }
}
