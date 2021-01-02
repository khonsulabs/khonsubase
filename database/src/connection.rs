use futures::executor::block_on;
use once_cell::sync::OnceCell;
use sqlx::PgPool;
use std::env;

static POOL: OnceCell<PgPool> = OnceCell::new();

pub async fn pool() -> &'static PgPool {
    POOL.get_or_init(|| {
        block_on(PgPool::connect(
            &env::var("DATABASE_URL").expect("DATABASE_URL not set"),
        ))
        .expect("Error initializing postgres pool")
    })
}
