//! # grind-domain
//!
//! Couche la plus interne (Clean Architecture). **Aucune** dépendance framework
//! (ni Axum, ni Leptos, ni SeaORM) — pur, testable et compilable en WASM.

pub mod entities;
pub mod error;
pub mod value_objects;

pub use error::DomainError;
