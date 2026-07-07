//! # grind-infrastructure
//!
//! Couche adapters (Clean Architecture) : implémente les ports de `grind-application`.
//! Pour l'instant : `security` (auth hybride PBKDF2 Django → argon2).
//! À venir (Phase 3) : `persistence` (SeaORM), `cache` (Redis).

pub mod security;

pub use security::{verify_django_pbkdf2, HashError, PasswordService, VerifyOutcome};
