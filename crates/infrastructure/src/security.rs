//! Hachage de mot de passe **hybride** :
//! - vérifie les hash Django existants (`pbkdf2_sha256$...`) → compat, zéro friction ;
//! - au 1er login réussi sur un hash legacy, signale un besoin de re-hash argon2 ;
//! - hache les nouveaux mots de passe en argon2 (état de l'art).

use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use pbkdf2::pbkdf2_hmac;
use rand_core::OsRng;
use sha2::Sha256;
use subtle::ConstantTimeEq;

#[derive(Debug, thiserror::Error)]
pub enum HashError {
    #[error("unknown password hash format")]
    UnknownFormat,
    #[error("malformed django pbkdf2 hash")]
    MalformedDjangoHash,
    #[error("argon2 error: {0}")]
    Argon2(String),
}

/// Résultat d'une vérification de mot de passe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOutcome {
    /// Mot de passe incorrect.
    Invalid,
    /// Correct, hash déjà au format cible (argon2) — rien à faire.
    Valid,
    /// Correct, mais hash legacy (Django PBKDF2) → à ré-écrire en argon2.
    ValidNeedsRehash,
}

/// Vérifie un mot de passe contre un hash Django `pbkdf2_sha256$<iter>$<salt>$<b64>`.
///
/// Django utilise le *salt* comme octets bruts (pas de base64-decode) et
/// base64-encode la clé dérivée (32 octets pour SHA-256).
pub fn verify_django_pbkdf2(password: &str, encoded: &str) -> Result<bool, HashError> {
    let mut parts = encoded.split('$');
    let algorithm = parts.next().ok_or(HashError::MalformedDjangoHash)?;
    let iterations = parts.next().ok_or(HashError::MalformedDjangoHash)?;
    let salt = parts.next().ok_or(HashError::MalformedDjangoHash)?;
    let hash_b64 = parts.next().ok_or(HashError::MalformedDjangoHash)?;
    if parts.next().is_some() {
        return Err(HashError::MalformedDjangoHash);
    }
    if algorithm != "pbkdf2_sha256" {
        // pbkdf2_sha1 / argon2 / bcrypt Django non gérés ici (cf. §migration).
        return Err(HashError::UnknownFormat);
    }

    let iterations: u32 = iterations.parse().map_err(|_| HashError::MalformedDjangoHash)?;
    let expected = STANDARD
        .decode(hash_b64)
        .map_err(|_| HashError::MalformedDjangoHash)?;

    let mut derived = vec![0u8; expected.len()];
    pbkdf2_hmac::<Sha256>(password.as_bytes(), salt.as_bytes(), iterations, &mut derived);

    Ok(bool::from(derived.ct_eq(&expected)))
}

/// Service de mot de passe injectable (implémentera le port `PasswordHasher`).
#[derive(Default)]
pub struct PasswordService {
    argon2: Argon2<'static>,
}

impl PasswordService {
    pub fn new() -> Self {
        Self::default()
    }

    /// Hache un nouveau mot de passe en argon2 (format PHC `$argon2id$...`).
    pub fn hash(&self, password: &str) -> Result<String, HashError> {
        let salt = SaltString::generate(&mut OsRng);
        let hash = self
            .argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| HashError::Argon2(e.to_string()))?;
        Ok(hash.to_string())
    }

    /// Vérifie un mot de passe contre un hash argon2 **ou** un hash Django legacy.
    pub fn verify(&self, password: &str, stored: &str) -> Result<VerifyOutcome, HashError> {
        if stored.starts_with("$argon2") {
            let parsed = PasswordHash::new(stored).map_err(|e| HashError::Argon2(e.to_string()))?;
            match self.argon2.verify_password(password.as_bytes(), &parsed) {
                Ok(()) => Ok(VerifyOutcome::Valid),
                Err(_) => Ok(VerifyOutcome::Invalid),
            }
        } else if stored.starts_with("pbkdf2_sha256$") {
            if verify_django_pbkdf2(password, stored)? {
                Ok(VerifyOutcome::ValidNeedsRehash)
            } else {
                Ok(VerifyOutcome::Invalid)
            }
        } else {
            Err(HashError::UnknownFormat)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hash Django RÉEL généré par `make_password('grind1234')` (Django 5, 720000 itérations).
    const DJANGO_HASH: &str =
        "pbkdf2_sha256$720000$0NIEqHeVS2mhpZNP9aCWwH$LGzPLXAIiHEXqaniXAAtLqsWm3Gg5+eWrfm+/goKBlQ=";

    #[test]
    fn verifies_real_django_hash() {
        assert!(verify_django_pbkdf2("grind1234", DJANGO_HASH).unwrap());
        assert!(!verify_django_pbkdf2("wrong-password", DJANGO_HASH).unwrap());
    }

    #[test]
    fn service_flags_legacy_hash_for_rehash() {
        let svc = PasswordService::new();
        assert_eq!(svc.verify("grind1234", DJANGO_HASH).unwrap(), VerifyOutcome::ValidNeedsRehash);
        assert_eq!(svc.verify("nope", DJANGO_HASH).unwrap(), VerifyOutcome::Invalid);
    }

    #[test]
    fn argon2_roundtrip_is_valid_without_rehash() {
        let svc = PasswordService::new();
        let hash = svc.hash("grind1234").unwrap();
        assert!(hash.starts_with("$argon2"));
        assert_eq!(svc.verify("grind1234", &hash).unwrap(), VerifyOutcome::Valid);
        assert_eq!(svc.verify("grind1235", &hash).unwrap(), VerifyOutcome::Invalid);
    }

    #[test]
    fn unknown_format_is_rejected() {
        let svc = PasswordService::new();
        assert!(matches!(svc.verify("x", "plaintext-oops"), Err(HashError::UnknownFormat)));
        assert!(matches!(
            verify_django_pbkdf2("x", "pbkdf2_sha1$1$salt$aGFzaA=="),
            Err(HashError::UnknownFormat)
        ));
    }
}
