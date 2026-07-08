//! Adapters du port `Cache` (couche interface adapters).
//! - `RedisCache` : implémentation Redis (prod), best-effort.
//! - `NoopCache` : implémentation nulle (dev/tests sans Redis).

use async_trait::async_trait;
use redis::AsyncCommands;

use grind_application::Cache;

/// Cache Redis. La connexion `MultiplexedConnection` est clonable et bon marché à
/// cloner : chaque opération en prend un clone (pas de pool à gérer).
#[derive(Clone)]
pub struct RedisCache {
    conn: redis::aio::MultiplexedConnection,
}

impl RedisCache {
    /// Ouvre une connexion et vérifie le lien avec un `PING`.
    /// `url` ex : `redis://redis:6379`.
    pub async fn connect(url: &str) -> Result<Self, redis::RedisError> {
        let client = redis::Client::open(url)?;
        let mut conn = client.get_multiplexed_async_connection().await?;
        redis::cmd("PING").query_async::<()>(&mut conn).await?;
        Ok(Self { conn })
    }
}

#[async_trait]
impl Cache for RedisCache {
    async fn get(&self, key: &str) -> Option<String> {
        let mut conn = self.conn.clone();
        // Échec réseau / clé absente → None (best-effort, on relira la source).
        conn.get::<_, Option<String>>(key).await.ok().flatten()
    }

    async fn set(&self, key: &str, value: &str, ttl_secs: u64) {
        let mut conn = self.conn.clone();
        // Silencieux : un échec d'écriture de cache ne doit pas remonter.
        let _: Result<(), _> = conn.set_ex(key, value, ttl_secs).await;
    }
}

/// Cache nul : ne stocke rien, `get` renvoie toujours `None`.
/// Utilisé en dev/tests quand `REDIS_URL` n'est pas configuré.
pub struct NoopCache;

#[async_trait]
impl Cache for NoopCache {
    async fn get(&self, _key: &str) -> Option<String> {
        None
    }
    async fn set(&self, _key: &str, _value: &str, _ttl_secs: u64) {}
}
