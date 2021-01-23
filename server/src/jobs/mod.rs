use std::{
    fmt::Debug,
    time::{Duration, Instant},
};

mod session_cleanup;

#[rocket::async_trait]
trait Job: Send + Sync + Debug + 'static {
    fn period(&self) -> Duration;
    async fn execute(&mut self) -> anyhow::Result<()>;

    fn instance(self) -> JobInstance
    where
        Self: Sized,
    {
        JobInstance::new(self)
    }
}

#[derive(Debug)]
pub(crate) struct JobInstance {
    job: Box<dyn Job>,
    last_executed: Option<Instant>,
}

impl JobInstance {
    fn new<T: Job>(job: T) -> Self {
        Self {
            job: Box::new(job),
            last_executed: None,
        }
    }

    fn is_ready(&self, now: Instant) -> bool {
        match self.last_executed {
            Some(_) => self.next_execution() < now,
            None => true,
        }
    }

    fn next_execution(&self) -> Instant {
        match self.last_executed {
            Some(last_executed) => last_executed + self.job.period(),
            None => Instant::now(),
        }
    }
}

pub async fn run() -> anyhow::Result<()> {
    let mut jobs = vec![session_cleanup::job()];

    loop {
        let now = Instant::now();
        for instance in jobs.iter_mut().filter(|i| i.is_ready(now)) {
            instance.job.execute().await?;
            instance.last_executed = Some(now);
        }

        let next_job = jobs.iter().map(|j| j.next_execution()).min().unwrap();
        tokio::time::delay_until(next_job.into()).await;
    }
}
