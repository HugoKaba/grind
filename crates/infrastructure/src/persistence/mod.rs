//! Couche persistence : entités SeaORM, migrations, repositories.

pub mod entities;
pub mod migration;
pub mod repositories;
pub mod seed;

use sea_orm::{ConnectOptions, Database, DatabaseConnection, DbErr};
use sea_orm_migration::MigratorTrait;

pub use migration::Migrator;

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

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{ConnectionTrait, Statement};

    #[tokio::test]
    async fn migrations_run_on_sqlite_and_tables_exist() {
        let db = connect_and_migrate("sqlite::memory:").await.unwrap();

        // Les tables doivent exister : une requête count ne doit pas échouer.
        for table in ["users", "sport", "team", "post", "follow", "post_like", "match_event", "team_follow"] {
            let sql = format!("SELECT COUNT(*) AS c FROM {table}");
            db.query_one(Statement::from_string(db.get_database_backend(), sql))
                .await
                .unwrap_or_else(|e| panic!("table {table} manquante: {e}"))
                .expect("une ligne de résultat");
        }
    }
}
