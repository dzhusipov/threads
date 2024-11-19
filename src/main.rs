
use actix_web::{web, App, HttpServer};

use dotenvy::dotenv;
use std::fs;
use std::sync::{Arc, Mutex};

use tokio_cron_scheduler::JobScheduler;

mod rest;
mod service;
mod models;

use models::app_state::AppState;
use service::service_utils::schedule_jobs;
use rest::rest_api::{get_image, run_job_endpoint};

#[actix_web::main]
async fn main() -> std::io::Result<()> {

    log4rs::init_file("config/log4rs.yaml", Default::default()).unwrap();
    // Load environment variables
    dotenv().ok();

    // Ensure IMAGE_FOLDER exists
    let image_folder = "downloaded_images";
    fs::create_dir_all(image_folder).expect("Failed to create image folder");

    // Start the scheduler in the background
    let scheduler = Arc::new(Mutex::new(JobScheduler::new().await.unwrap()));

    // Schedule the jobs
    schedule_jobs(scheduler.clone()).await;

    // Start the Actix_web server
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(AppState {
                image_folder: image_folder.to_string(),
                scheduler: scheduler.clone(),
            }))
            .service(get_image)
            .service(run_job_endpoint)
    })
    .bind(("0.0.0.0", 19998))?
    .run()
    .await
}
