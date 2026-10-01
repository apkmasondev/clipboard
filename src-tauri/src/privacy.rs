use crate::model::Settings;
pub fn blocked_source(settings: &Settings, source: &str) -> bool {
    settings.paused
        || (source.is_empty() && settings.block_unknown)
        || settings
            .blocked_apps
            .iter()
            .any(|s| s.eq_ignore_ascii_case(source))
}
pub fn likely_secret(text: &str) -> bool {
    let t = text.trim();
    if t.contains("-----BEGIN PRIVATE KEY-----")
        || t.contains("-----BEGIN RSA PRIVATE KEY-----")
        || t.contains("-----BEGIN OPENSSH PRIVATE KEY-----")
    {
        return true;
    }
    if t.contains(char::is_whitespace) || t.len() < 20 {
        return false;
    }
    if [
        "sk-proj-",
        "sk-ant-",
        "ghp_",
        "github_pat_",
        "gho_",
        "xoxb-",
        "xoxp-",
        "AKIA",
        "ASIA",
    ]
    .iter()
    .any(|p| t.starts_with(p))
    {
        return true;
    }
    let parts: Vec<_> = t.split('.').collect();
    parts.len() == 3
        && t.starts_with("eyJ")
        && parts.iter().all(|p| {
            p.len() > 8
                && p.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn conservative_detection() {
        assert!(likely_secret("ghp_012345678901234567890123456789"));
        assert!(likely_secret("-----BEGIN PRIVATE KEY-----\nabc"));
        for t in [
            "correct horse battery staple",
            "this_is_a_long_variable_name",
            "const key = process.env.API_KEY;",
            "https://example.com/very-long-path",
            "sk-proj- example explanation",
        ] {
            assert!(!likely_secret(t), "{t}");
        }
    }
    #[test]
    fn source_rules() {
        let s = Settings::default();
        assert!(blocked_source(&s, "KeePass.EXE"));
        assert!(!blocked_source(&s, "notepad.exe"));
    }
}
