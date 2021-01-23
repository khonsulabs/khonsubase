use crate::jobs::{Job, JobInstance};
use database::schema::accounts::Session;
use tokio::time::Duration;

#[derive(Debug)]
struct SessionCleanup;

#[rocket::async_trait]
impl Job for SessionCleanup {
    fn period(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn execute(&mut self) -> anyhow::Result<()> {
        let sessions_expired = Session::cleanup(database::pool()).await?;

        info!("SessionCleanup expired {} sessions", sessions_expired);

        Ok(())
    }
}

pub(crate) fn job() -> JobInstance {
    SessionCleanup.instance()
}
