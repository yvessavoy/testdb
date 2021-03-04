use async_std::{fs::read_to_string, task::block_on};
use testdb::*;
use sqlx::Postgres;

#[async_std::test]
async fn test_setup() {
    let url = env!("DATABASE_URL");
    let testdb = TestDb::new(url, |mut connection| {
        futures::executor::block_on(async {
            let statements = async_std::fs::read_to_string("tests/schema_postgres.sql").await.unwrap();
            for statement in statements.split(";") {
                sqlx::query::<Postgres>(&statement)
                .execute(&mut connection)
                .await
                .unwrap();
            }
        });
    }).await;
}