//! Entités du domaine sport. Les invariants inter-champs sont garantis
//! par les constructeurs `new(...)` qui renvoient `Result<_, DomainError>`.

use crate::error::DomainError;
use crate::value_objects::PostContent;

macro_rules! id_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub i64);
    };
}

id_newtype!(UserId);
id_newtype!(SportId);
id_newtype!(TeamId);
id_newtype!(MatchId);
id_newtype!(PostId);

/// Discipline sportive (reference data).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sport {
    pub id: SportId,
    pub name: String,
    pub slug: String,
}

/// Équipe / club, rattaché à un sport (reference data).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    pub id: TeamId,
    pub sport: SportId,
    pub name: String,
    pub slug: String,
    pub country: String,
}

/// Profil sportif d'un utilisateur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AthleteProfile {
    pub user: UserId,
    pub sport: SportId,
    pub team: Option<TeamId>,
    pub position: Option<String>,
    pub is_pro: bool,
    pub jersey_number: Option<u8>,
}

impl AthleteProfile {
    /// Invariant : un athlète professionnel appartient forcément à une équipe.
    pub fn new(
        user: UserId,
        sport: SportId,
        team: Option<TeamId>,
        position: Option<String>,
        is_pro: bool,
        jersey_number: Option<u8>,
    ) -> Result<Self, DomainError> {
        if is_pro && team.is_none() {
            return Err(DomainError::ProWithoutTeam);
        }
        Ok(Self { user, sport, team, position, is_pro, jersey_number })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    Scheduled,
    Live,
    Finished,
}

/// Rencontre entre deux équipes (support du live-posting).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    pub id: MatchId,
    pub sport: SportId,
    pub home: TeamId,
    pub away: TeamId,
    pub kickoff_unix: i64,
    pub status: MatchStatus,
}

impl Match {
    /// Invariants : deux équipes distinctes, du même sport que le match.
    pub fn new(
        id: MatchId,
        home: &Team,
        away: &Team,
        kickoff_unix: i64,
        status: MatchStatus,
    ) -> Result<Self, DomainError> {
        if home.id == away.id {
            return Err(DomainError::SameTeamMatch);
        }
        if home.sport != away.sport {
            return Err(DomainError::CrossSportMatch);
        }
        Ok(Self {
            id,
            sport: home.sport,
            home: home.id,
            away: away.id,
            kickoff_unix,
            status,
        })
    }
}

/// Post (ex-`Tweet`), avec contexte sportif optionnel.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    pub id: PostId,
    pub author: UserId,
    pub content: PostContent,
    pub parent: Option<PostId>,
    pub sport: Option<SportId>,
    pub match_id: Option<MatchId>,
    pub team: Option<TeamId>,
}

impl Post {
    pub fn is_reply(&self) -> bool {
        self.parent.is_some()
    }
}

/// Relation de suivi entre deux utilisateurs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Follow {
    pub follower: UserId,
    pub following: UserId,
}

impl Follow {
    /// Invariant : on ne peut pas se suivre soi-même.
    pub fn new(follower: UserId, following: UserId) -> Result<Self, DomainError> {
        if follower == following {
            return Err(DomainError::SelfFollow);
        }
        Ok(Self { follower, following })
    }
}

/// Un utilisateur suit une équipe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TeamFollow {
    pub user: UserId,
    pub team: TeamId,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn team(id: i64, sport: i64) -> Team {
        Team {
            id: TeamId(id),
            sport: SportId(sport),
            name: format!("Team {id}"),
            slug: format!("team-{id}"),
            country: "FR".into(),
        }
    }

    #[test]
    fn follow_rejects_self() {
        assert_eq!(Follow::new(UserId(1), UserId(1)).unwrap_err(), DomainError::SelfFollow);
        assert!(Follow::new(UserId(1), UserId(2)).is_ok());
    }

    #[test]
    fn pro_athlete_requires_team() {
        let err = AthleteProfile::new(UserId(1), SportId(1), None, None, true, None).unwrap_err();
        assert_eq!(err, DomainError::ProWithoutTeam);
        assert!(AthleteProfile::new(UserId(1), SportId(1), Some(TeamId(9)), None, true, Some(10)).is_ok());
        // amateur without team is fine
        assert!(AthleteProfile::new(UserId(2), SportId(1), None, None, false, None).is_ok());
    }

    #[test]
    fn match_requires_two_distinct_same_sport_teams() {
        let psg = team(1, 1);
        let om = team(2, 1);
        let lakers = team(3, 2);

        assert!(Match::new(MatchId(1), &psg, &om, 0, MatchStatus::Scheduled).is_ok());
        assert_eq!(
            Match::new(MatchId(2), &psg, &psg, 0, MatchStatus::Scheduled).unwrap_err(),
            DomainError::SameTeamMatch
        );
        assert_eq!(
            Match::new(MatchId(3), &psg, &lakers, 0, MatchStatus::Scheduled).unwrap_err(),
            DomainError::CrossSportMatch
        );
    }
}
