//! Value objects : garantissent leurs invariants à la construction.

use crate::error::DomainError;

/// Contenu d'un post : 1..=280 caractères (compté en `char`, pas en octets).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PostContent(String);

impl PostContent {
    pub const MAX: usize = 280;

    pub fn new(raw: impl Into<String>) -> Result<Self, DomainError> {
        let s = raw.into();
        let trimmed = s.trim();
        let len = trimmed.chars().count();
        if len == 0 || len > Self::MAX {
            return Err(DomainError::InvalidPostContent);
        }
        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Extrait les hashtags (`#Football`) sans dépendance regex.
    /// Retourne les libellés en minuscules, sans le `#`, dédupliqués dans l'ordre.
    pub fn hashtags(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut chars = self.0.chars().peekable();
        while let Some(c) = chars.next() {
            if c != '#' {
                continue;
            }
            let mut tag = String::new();
            while let Some(&n) = chars.peek() {
                if n.is_alphanumeric() || n == '_' {
                    tag.push(n.to_ascii_lowercase());
                    chars.next();
                } else {
                    break;
                }
            }
            if !tag.is_empty() && !out.contains(&tag) {
                out.push(tag);
            }
        }
        out
    }
}

/// Nom d'utilisateur : 1..=30 caractères `[a-z0-9_]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Username(String);

impl Username {
    pub fn new(raw: impl Into<String>) -> Result<Self, DomainError> {
        let s = raw.into();
        let len = s.chars().count();
        let valid_chars = s
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if (1..=30).contains(&len) && valid_chars {
            Ok(Self(s))
        } else {
            Err(DomainError::InvalidUsername)
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Slugifie un libellé ("Champions League" -> "champions-league").
pub fn slugify(raw: &str) -> String {
    let mut slug = String::new();
    let mut prev_dash = false;
    for c in raw.trim().chars() {
        if c.is_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
            prev_dash = false;
        } else if !prev_dash && !slug.is_empty() {
            slug.push('-');
            prev_dash = true;
        }
    }
    slug.trim_end_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_content_rejects_empty_and_too_long() {
        assert_eq!(PostContent::new("   ").unwrap_err(), DomainError::InvalidPostContent);
        let long = "a".repeat(281);
        assert_eq!(PostContent::new(long).unwrap_err(), DomainError::InvalidPostContent);
        assert!(PostContent::new("a".repeat(280)).is_ok());
    }

    #[test]
    fn post_content_counts_chars_not_bytes() {
        // 280 emojis = 280 chars but > 280 bytes; must be accepted.
        assert!(PostContent::new("⚽".repeat(280)).is_ok());
        assert!(PostContent::new("⚽".repeat(281)).is_err());
    }

    #[test]
    fn hashtags_are_extracted_lowercased_and_deduped() {
        let c = PostContent::new("Big win! #Football #GOALS #football").unwrap();
        assert_eq!(c.hashtags(), vec!["football".to_string(), "goals".to_string()]);
    }

    #[test]
    fn username_rules() {
        assert!(Username::new("messi").is_ok());
        assert!(Username::new("king_10").is_ok());
        assert!(Username::new("Messi").is_err()); // uppercase
        assert!(Username::new("").is_err());
        assert!(Username::new("a".repeat(31)).is_err());
    }

    #[test]
    fn slugify_basic() {
        assert_eq!(slugify("Champions League"), "champions-league");
        assert_eq!(slugify("  Réal  Madrid!! "), "réal-madrid");
    }
}
