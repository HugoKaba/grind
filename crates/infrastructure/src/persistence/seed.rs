//! Backfill / seed : reference data (sports, équipes) + athlètes rattachés.
//! Remplace `seed_sports_data.py`. Mots de passe hachés en argon2 via le service auth.

use sea_orm::{ActiveModelTrait, DatabaseConnection, DbErr, Set};

use super::entities::{athlete_profile, sport, team, users};
use crate::security::PasswordService;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeedReport {
    pub sports: u64,
    pub teams: u64,
    pub athletes: u64,
}

/// Seed (à lancer sur une base vide / de test).
pub async fn seed_reference_and_athletes(
    db: &DatabaseConnection,
    hasher: &PasswordService,
) -> Result<SeedReport, DbErr> {
    // --- Sport ---
    let football = sport::ActiveModel {
        name: Set("Football".to_owned()),
        slug: Set("football".to_owned()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    // --- Teams ---
    let inter_miami = team::ActiveModel {
        sport_id: Set(football.id),
        name: Set("Inter Miami CF".to_owned()),
        slug: Set("inter-miami".to_owned()),
        country: Set("US".to_owned()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    let al_nassr = team::ActiveModel {
        sport_id: Set(football.id),
        name: Set("Al Nassr FC".to_owned()),
        slug: Set("al-nassr".to_owned()),
        country: Set("SA".to_owned()),
        ..Default::default()
    }
    .insert(db)
    .await?;

    // --- Athletes (ex-`seed_sports_data.py`), rattachés à leur équipe ---
    let athletes = [
        ("messi", "Lionel Messi", inter_miami.id, 10_i16),
        ("ronaldo", "Cristiano Ronaldo", al_nassr.id, 7_i16),
    ];

    for (username, display_name, team_id, jersey) in athletes {
        let password = hasher
            .hash("grind1234")
            .map_err(|e| DbErr::Custom(format!("hash error: {e}")))?;

        let user = users::ActiveModel {
            username: Set(username.to_owned()),
            password: Set(password),
            display_name: Set(display_name.to_owned()),
            ..Default::default()
        }
        .insert(db)
        .await?;

        athlete_profile::ActiveModel {
            user_id: Set(user.id),
            sport_id: Set(football.id),
            team_id: Set(Some(team_id)),
            position: Set(Some("Forward".to_owned())),
            is_pro: Set(true),
            jersey_number: Set(Some(jersey)),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }

    Ok(SeedReport { sports: 1, teams: 2, athletes: 2 })
}
