//! Migrations SeaORM (schéma portable SQLite/Postgres via le builder).
//! Couvre les tables micro-blog + les tables sport (§2.1-bis du plan).
//! FKs volontairement omises ici pour la portabilité SQLite des tests ;
//! elles seront ajoutées dans une migration Postgres dédiée (cf. plan Phase 3).

use sea_orm_migration::prelude::*;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(m0001_init::Migration)]
    }
}

/// Petit helper : `Alias::new(s)` pour construire des identifiants sans enums Iden verbeux.
fn a(s: &str) -> Alias {
    Alias::new(s)
}

mod m0001_init {
    use super::a;
    use sea_orm_migration::prelude::*;

    pub struct Migration;

    impl MigrationName for Migration {
        fn name(&self) -> &str {
            "m0001_init"
        }
    }

    #[async_trait::async_trait]
    impl MigrationTrait for Migration {
        async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
            // users
            m.create_table(
                Table::create()
                    .table(a("users"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("username")).string().not_null().unique_key())
                    .col(ColumnDef::new(a("password")).string().not_null())
                    .col(ColumnDef::new(a("display_name")).string().not_null().default(""))
                    .to_owned(),
            )
            .await?;

            // sport (reference data)
            m.create_table(
                Table::create()
                    .table(a("sport"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("name")).string().not_null())
                    .col(ColumnDef::new(a("slug")).string().not_null().unique_key())
                    .to_owned(),
            )
            .await?;

            // team (reference data)
            m.create_table(
                Table::create()
                    .table(a("team"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("sport_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("name")).string().not_null())
                    .col(ColumnDef::new(a("slug")).string().not_null().unique_key())
                    .col(ColumnDef::new(a("country")).string().not_null().default(""))
                    .to_owned(),
            )
            .await?;

            // athlete_profile
            m.create_table(
                Table::create()
                    .table(a("athlete_profile"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("user_id")).big_integer().not_null().unique_key())
                    .col(ColumnDef::new(a("sport_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("team_id")).big_integer().null())
                    .col(ColumnDef::new(a("position")).string().null())
                    .col(ColumnDef::new(a("is_pro")).boolean().not_null().default(false))
                    .col(ColumnDef::new(a("jersey_number")).small_integer().null())
                    .to_owned(),
            )
            .await?;

            // match_event (rencontre — support live-posting)
            m.create_table(
                Table::create()
                    .table(a("match_event"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("sport_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("home_team_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("away_team_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("kickoff_at")).timestamp_with_time_zone().not_null())
                    .col(ColumnDef::new(a("status")).string().not_null().default("scheduled"))
                    .col(ColumnDef::new(a("home_score")).integer().null())
                    .col(ColumnDef::new(a("away_score")).integer().null())
                    .to_owned(),
            )
            .await?;

            // post (ex-tweet) + contexte sportif + compteurs dénormalisés
            m.create_table(
                Table::create()
                    .table(a("post"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("author_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("content")).string().not_null())
                    .col(ColumnDef::new(a("parent_id")).big_integer().null())
                    .col(ColumnDef::new(a("sport_id")).big_integer().null())
                    .col(ColumnDef::new(a("match_id")).big_integer().null())
                    .col(ColumnDef::new(a("team_id")).big_integer().null())
                    .col(ColumnDef::new(a("likes_count")).big_integer().not_null().default(0))
                    .col(ColumnDef::new(a("reposts_count")).big_integer().not_null().default(0))
                    .col(ColumnDef::new(a("replies_count")).big_integer().not_null().default(0))
                    .col(ColumnDef::new(a("created_at")).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;
            m.create_index(
                Index::create().if_not_exists().name("idx_post_author").table(a("post")).col(a("author_id")).to_owned(),
            )
            .await?;
            m.create_index(
                Index::create().if_not_exists().name("idx_post_created").table(a("post")).col(a("created_at")).to_owned(),
            )
            .await?;

            // follow (unique follower+following)
            m.create_table(
                Table::create()
                    .table(a("follow"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("follower_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("following_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("created_at")).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;
            m.create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("uq_follow_pair")
                    .table(a("follow"))
                    .col(a("follower_id"))
                    .col(a("following_id"))
                    .to_owned(),
            )
            .await?;

            // post_like (unique user+post)
            m.create_table(
                Table::create()
                    .table(a("post_like"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("user_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("post_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("created_at")).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;
            m.create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("uq_like_pair")
                    .table(a("post_like"))
                    .col(a("user_id"))
                    .col(a("post_id"))
                    .to_owned(),
            )
            .await?;

            // team_follow (unique user+team)
            m.create_table(
                Table::create()
                    .table(a("team_follow"))
                    .if_not_exists()
                    .col(ColumnDef::new(a("id")).big_integer().not_null().auto_increment().primary_key())
                    .col(ColumnDef::new(a("user_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("team_id")).big_integer().not_null())
                    .col(ColumnDef::new(a("created_at")).timestamp_with_time_zone().not_null())
                    .to_owned(),
            )
            .await?;
            m.create_index(
                Index::create()
                    .if_not_exists()
                    .unique()
                    .name("uq_team_follow_pair")
                    .table(a("team_follow"))
                    .col(a("user_id"))
                    .col(a("team_id"))
                    .to_owned(),
            )
            .await?;

            Ok(())
        }

        async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
            for t in [
                "team_follow",
                "post_like",
                "follow",
                "post",
                "match_event",
                "athlete_profile",
                "team",
                "sport",
                "users",
            ] {
                m.drop_table(Table::drop().table(a(t)).if_exists().to_owned()).await?;
            }
            Ok(())
        }
    }
}
