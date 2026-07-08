//! Couche persistence : entités SeaORM, migrations, repositories.

pub mod entities;
pub mod migration;
pub mod repositories;
pub mod seed;

use sea_orm::{ConnectOptions, Database, DbErr};
use sea_orm_migration::MigratorTrait;

pub use migration::Migrator;
pub use repositories::{
    SeaOrmBookmarkRepository, SeaOrmFeedRepository, SeaOrmFollowRepository, SeaOrmHashtagRepository,
    SeaOrmLikeRepository, SeaOrmMatchRepository, SeaOrmMessageRepository,
    SeaOrmNotificationRepository, SeaOrmPostRepository, SeaOrmRepostRepository, SeaOrmSportCatalog,
    SeaOrmTeamFollowRepository, SeaOrmUserRepository,
};
pub use sea_orm::DatabaseConnection;

/// Ouvre une connexion et applique les migrations. `url` ex :
/// `postgres://user:pass@host/db` ou `sqlite::memory:`.
pub async fn connect_and_migrate(url: &str) -> Result<DatabaseConnection, DbErr> {
    let mut opt = ConnectOptions::new(url.to_owned());
    // SQLite `:memory:` = une DB par connexion → forcer 1 connexion pour que
    // le schéma migré reste visible sur toute la durée (tests).
    if url.starts_with("sqlite::memory:") {
        opt.max_connections(1);
    }
    let db = Database::connect(opt).await?;
    Migrator::up(&db, None).await?;
    Ok(db)
}

/// `true` si la base ne contient aucun utilisateur (→ à seeder). Permet un
/// démarrage **idempotent** sur une BDD persistante (Postgres) : on ne seede
/// qu'une fois, sans dupliquer à chaque redémarrage du conteneur.
pub async fn is_empty(db: &DatabaseConnection) -> Result<bool, DbErr> {
    use entities::users;
    use sea_orm::{EntityTrait, PaginatorTrait};
    Ok(users::Entity::find().count(db).await? == 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Statement};

    #[tokio::test]
    async fn migrations_run_on_sqlite_and_tables_exist() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();

        // Les tables doivent exister : une requête count ne doit pas échouer.
        for table in ["users", "sport", "team", "post", "follow", "post_like", "match_event", "team_follow", "repost", "bookmark", "message", "notification", "hashtag", "post_hashtag"] {
            let sql = format!("SELECT COUNT(*) AS c FROM {table}");
            db.query_one(Statement::from_string(db.get_database_backend(), sql))
                .await
                .unwrap_or_else(|e| panic!("table {table} manquante: {e}"))
                .expect("une ligne de résultat");
        }
    }

    /// Valide la portabilité **PostgreSQL** des migrations + du seed + de l'idempotence.
    /// Ne s'exécute que si `TEST_DATABASE_URL` pointe vers un Postgres jetable :
    /// `TEST_DATABASE_URL=postgres://grind@localhost:55432/grind cargo test -p grind-infrastructure`.
    #[tokio::test]
    async fn migrations_and_seed_run_on_postgres_when_configured() {
        let Ok(url) = std::env::var("TEST_DATABASE_URL") else {
            eprintln!("TEST_DATABASE_URL absent → test Postgres ignoré");
            return;
        };
        let db = connect_and_migrate(&url).await.expect("connexion + migrations Postgres");

        // Base neuve → vide, puis seed, puis plus vide (idempotence).
        assert!(is_empty(&db).await.unwrap(), "base neuve doit être vide");
        let report = crate::persistence::seed::seed_reference_and_athletes(
            &db,
            &crate::security::PasswordService::new(),
        )
        .await
        .expect("seed Postgres");
        assert_eq!(report.athletes, 2);
        assert!(!is_empty(&db).await.unwrap(), "base seedée n'est plus vide");
    }
}
