use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::JobScheduler;

#[allow(dead_code)]
pub struct AppState {
    pub image_folder: String,
    pub scheduler: Arc<Mutex<JobScheduler>>,
}