//! Erreurs métier pures — aucune dépendance framework.

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DomainError {
    #[error("post content must be 1..=280 characters")]
    InvalidPostContent,

    #[error("username must be 1..=30 chars, [a-z0-9_]")]
    InvalidUsername,

    #[error("a user cannot follow themselves")]
    SelfFollow,

    #[error("a match must be between two different teams")]
    SameTeamMatch,

    #[error("a match must be between teams of the same sport")]
    CrossSportMatch,

    #[error("a professional athlete must belong to a team")]
    ProWithoutTeam,
}
