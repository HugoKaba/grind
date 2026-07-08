//! # grind-infrastructure
//!
//! Couche adapters (Clean Architecture) : implémente les ports de `grind-application`.
//! - `security` : auth hybride PBKDF2 Django → argon2 ;
//! - `persistence` : SeaORM (PostgreSQL/SQLite) ;
//! - `cache` : Redis (`RedisCache`) + `NoopCache`.

pub mod cache;
pub mod persistence;
pub mod security;

pub use cache::{NoopCache, RedisCache};
pub use security::{verify_django_pbkdf2, HashError, PasswordService, VerifyOutcome};
