use serde_json::json;
use tokio::task;
use actix_files::NamedFile;
use actix_web::{get, web, HttpRequest, HttpResponse, Responder};

use std::path::PathBuf;

use crate::models::app_state::AppState;
use crate::service::service_utils::job;
#[get("/images/{filename}")]
async fn get_image(
    state: web::Data<AppState>,
    path: web::Path<String>,
    req: HttpRequest, // Accept HttpRequest as a parameter
) -> impl Responder {
    let filename = path.into_inner();
    let filepath = PathBuf::from(&state.image_folder).join(&filename);

    if filepath.exists() {
        match NamedFile::open(filepath) {
            Ok(file) => file.into_response(&req), // Pass &req instead of &HttpResponse::Ok()
            Err(_) => HttpResponse::InternalServerError().body("Error opening file"),
        }
    } else {
        HttpResponse::NotFound().body("Image not found")
    }
}

#[get("/job")]
async fn run_job_endpoint(state: web::Data<AppState>) -> impl Responder {
    // Run the job asynchronously
    let image_folder = state.image_folder.clone();
    task::spawn(async move {
        job(&image_folder.clone(), None).await;
    });

    HttpResponse::Ok().json(json!({"message": "Job executed successfully"}))
}
