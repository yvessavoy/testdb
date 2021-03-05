use sqlx::Connection;
use sqlx::PgConnection;
use sqlx::PgPool;
use sqlx::Postgres;

#[derive(Debug)]
pub struct TestDb {
    db_url: String,
    db_pool: Option<PgPool>,
    connection: String,
    name: String,
}

/// Sets up a new DB for running tests with.
impl TestDb {
    pub async fn new<F: FnOnce(PgConnection) -> ()>(database_url: &str, setup_fn: F) -> Self {
        let db_url = generate_random_db_url(database_url);

        let (pg_conn, db_name) = split_database_url(&db_url);

        // Create the database
        let mut conn = PgConnection::connect(pg_conn).await.unwrap();
        let sql = format!(r#"CREATE DATABASE "{}""#, &db_name);
        sqlx::query::<Postgres>(&sql)
            .execute(&mut conn)
            .await
            .unwrap();

        // Run the user provided setup fn with a connection
        setup_fn(PgConnection::connect(&db_url).await.unwrap());

        Self {
            db_url: db_url.clone(),
            db_pool: Some(PgPool::connect(&db_url).await.unwrap()),
            connection: pg_conn.to_string(),
            name: db_name.to_string(),
        }
    }

    pub fn url(&self) -> String {
        self.db_url.clone()
    }

    pub fn db(&self) -> PgPool {
        self.db_pool.clone().unwrap()
    }
}

impl Drop for TestDb {
    fn drop(&mut self) {
        // Drop the DB Pool
        let _ = self.db_pool.take();

        // Disconnect all client and drop the database
        futures::executor::block_on(drop_db(&self.db_url, &self.connection, &self.name));
    }
}

fn split_database_url(db_url: &str) -> (&str, &str) {
    let separator_pos = db_url.rfind("/").unwrap();
    let pg_conn = &db_url[..=separator_pos];
    let db_name = &db_url[separator_pos + 1..];

    (pg_conn, db_name)
}

fn generate_random_db_url(database_url: &str) -> String {
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    // Set up the database per tests
    let rng = thread_rng();
    let suffix: String = rng
        .sample_iter(&Alphanumeric)
        .take(16)
        .map(char::from)
        .collect();
    format!("{}_{}", database_url, suffix)
}

async fn drop_db(db_url: &str, pg_conn: &str, db_name: &str) {
    let mut conn = PgConnection::connect(pg_conn).await.unwrap();

    // Disconnect any existing connections to the DB
    let sql = format!(
        r#"SELECT pg_terminate_backend(pg_stat_activity.pid)
FROM pg_stat_activity
WHERE pg_stat_activity.datname = '{db}'
AND pid <> pg_backend_pid();"#,
        db = db_name
    );
    sqlx::query::<Postgres>(&sql)
        .execute(&mut conn)
        .await
        .unwrap();

    // Clean it up, bubye!
    let sql = format!(r#"DROP DATABASE "{db}";"#, db = db_name);
    sqlx::query::<Postgres>(&sql)
        .execute(&mut conn)
        .await
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[async_std::test]
    async fn test_split_database_url() {
        let url = "postgres://user:password@host/testdb";
        let (conn, name) = split_database_url(url);
        assert_eq!(conn, "postgres://user:password@host/");
        assert_eq!(name, "testdb");
    }
}
