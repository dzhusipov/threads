
use chrono::{Local, NaiveTime, Timelike};
use log::{error, info};
use rand::seq::SliceRandom;
use rand::thread_rng;
use reqwest::Client;
use serde_json::json;
use std::env;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_cron_scheduler::{Job, JobScheduler};

pub async fn schedule_jobs(scheduler: Arc<Mutex<JobScheduler>>) {
    // Define the times when the job should run (24-hour format)
    let run_times = vec![
        NaiveTime::from_hms_opt(8, 0, 0).expect("Invalid time"),
        NaiveTime::from_hms_opt(14, 0, 0).expect("Invalid time"),
        NaiveTime::from_hms_opt(20, 0, 0).expect("Invalid time"),
    ];


    for run_time in run_times {
        let scheduler = scheduler.clone();
        let image_folder = "downloaded_images".to_string();
        let cron_expr = format!(
            "0 {} {} * * *",
            run_time.minute(),
            run_time.hour()
        );

        let job = Job::new_async(cron_expr.as_str(), move |_uuid, _l| {
            let image_folder = image_folder.clone();
            Box::pin(async move {
                info!("Scheduled job started");
                job(&image_folder, None).await;
                info!("Scheduled job finished");
            })
        })
        .unwrap();

        scheduler.lock().unwrap().add(job).await.unwrap();
    }

    scheduler.lock().unwrap().start().await.unwrap();
}

pub async fn job(image_folder: &str, use_image_url: Option<String>) {
    info!("Job started");
    if let Some(use_image_url) = use_image_url {
        // Use image from the provided URL
        if let Some(image_url) = process_image_from_url(&use_image_url, image_folder).await {
            let post_text = format!("Image from URL: {}", use_image_url);
            post_to_threads(&image_url, &post_text).await;
        } else {
            error!("Failed to process image from URL");
        }
    } else {
        // Generate a new prompt and image
        let prompt = generate_random_prompt().await;
        if let Some(image_url) = generate_image(&prompt).await {
            if let Some(processed_image_url) = process_image_from_url(&image_url, image_folder).await
            {
                post_to_threads(&processed_image_url, &prompt).await;
            } else {
                error!("Failed to process generated image");
            }
        } else {
            error!("Failed to generate image");
        }
    }
    info!("Job finished");
}

async fn generate_random_prompt() -> String {
    let initial_prompt = {
        let adjectives = vec![
            "beautiful",
            "serene",
            "mystical",
            "vibrant",
            "tranquil",
            "majestic",
            "ethereal",
            "enigmatic",
        ];
        let subjects = vec![
            "forest",
            "mountain",
            "ocean",
            "desert",
            "waterfall",
            "sky",
            "galaxy",
            "island",
        ];
        let styles = vec![
            "digital painting",
            "photorealistic",
            "abstract",
            "impressionist",
            "surreal",
            "minimalist",
            "fantasy",
        ];
        let times_of_day = vec![
            "at sunrise",
            "at sunset",
            "under the stars",
            "during a storm",
            "on a foggy morning",
            "in autumn",
        ];

        let mut rng = thread_rng();
        let adjective = adjectives.choose(&mut rng).unwrap();
        let subject = subjects.choose(&mut rng).unwrap();
        let style = styles.choose(&mut rng).unwrap();
        let time_of_day = times_of_day.choose(&mut rng).unwrap();

        let initial_prompt = format!("A {} {} {}, {}", adjective, subject, time_of_day, style);
        info!("Generated initial prompt: {}", initial_prompt);

        // Limit the scope of rng to this block
        initial_prompt
    };

    // Enrich the prompt using OpenAI GPT-4
    match get_enriched_prompt(&initial_prompt).await {
        Some(enriched_prompt) => {
            info!("Enriched prompt: {}", enriched_prompt);
            enriched_prompt
        }
        None => {
            error!("Using initial prompt as enrichment failed.");
            initial_prompt
        }
    }
}

async fn get_enriched_prompt(prompt: &str) -> Option<String> {
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = Client::new();

    let system_message = "You are a creative assistant that enriches image prompts for art generation. Make the following prompt more detailed and vivid, suitable for generating images with DALL·E.";
    let user_message = format!("Please enrich this prompt for an image: '{}'", prompt);

    let request_body = json!({
        "model": "gpt-4",
        "messages": [
            {"role": "system", "content": system_message},
            {"role": "user", "content": user_message},
        ],
        "temperature": 0.7,
        "max_tokens": 60,
    });

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(openai_api_key)
        .json(&request_body)
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        let enriched_prompt = json["choices"][0]["message"]["content"]
                            .as_str()
                            .map(|s| s.trim().to_string());
                        enriched_prompt
                    }
                    Err(e) => {
                        error!("Error parsing OpenAI response JSON: {}", e);
                        None
                    }
                }
            } else {
                error!("OpenAI API returned error: {}", response.status());
                None
            }
        }
        Err(e) => {
            error!("Error calling OpenAI API: {}", e);
            None
        }
    }
}

async fn generate_image(prompt: &str) -> Option<String> {
    info!("Starting image generation with prompt: {}", prompt);
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");
    let client = Client::new();

    let request_body = json!({
        "prompt": prompt,
        "n": 1,
        "size": "1024x1024",
    });

    let res = client
        .post("https://api.openai.com/v1/images/generations")
        .bearer_auth(openai_api_key)
        .json(&request_body)
        .send()
        .await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        if let Some(image_url) = json["data"][0]["url"].as_str() {
                            info!("Image generated successfully. URL: {}", image_url);
                            Some(image_url.to_string())
                        } else {
                            error!("Image URL not found in response");
                            None
                        }
                    }
                    Err(e) => {
                        error!("Error parsing OpenAI response JSON: {}", e);
                        None
                    }
                }
            } else {
                error!("OpenAI API returned error: {}", response.status());
                None
            }
        }
        Err(e) => {
            error!("Error calling OpenAI API: {}", e);
            None
        }
    }
}

async fn process_image_from_url(image_url: &str, image_folder: &str) -> Option<String> {
    info!("Downloading image from URL: {}", image_url);
    let client = Client::new();

    let res = client.get(image_url).send().await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                let bytes = response.bytes().await;
                match bytes {
                    Ok(bytes) => {
                        info!("Image downloaded successfully.");
                        // Open the image
                        match image::load_from_memory(&bytes) {
                            Ok(img) => {
                                // Save the image locally
                                let filename = format!(
                                    "image_{}.png",
                                    Local::now().format("%Y%m%d_%H%M%S")
                                );
                                let filepath = PathBuf::from(image_folder).join(&filename);

                                match img.save(&filepath) {
                                    Ok(_) => {
                                        info!("Image saved locally as: {:?}", filepath);
                                        // Construct the public URL for the image
                                        let public_image_url = format!(
                                            "https://threads.dasm.asia/images/{}",
                                            filename
                                        );
                                        Some(public_image_url)
                                    }
                                    Err(e) => {
                                        error!("Failed to save image: {}", e);
                                        None
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to open image: {}", e);
                                None
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to get image bytes: {}", e);
                        None
                    }
                }
            } else {
                error!("Failed to download image. Status code: {}", response.status());
                None
            }
        }
        Err(e) => {
            error!("Error downloading image: {}", e);
            None
        }
    }
}

async fn post_to_threads(image_url: &str, text: &str) {
    info!("Starting post to Threads with image URL: {}", image_url);
    info!("Text: {}", text);
    let threads_user_id = env::var("THREADS_USER_ID").expect("THREADS_USER_ID not set");
    let access_token = env::var("ACCESS_TOKEN").expect("ACCESS_TOKEN not set");
    let client = Client::new();

    // Step 1: Create the media container
    let threads_api_url = format!(
        "https://graph.threads.net/v1.0/{}/threads",
        threads_user_id
    );
    let params = [
        ("media_type", "IMAGE"),
        ("image_url", image_url),
        ("access_token", &access_token),
    ];

    let res = client.post(&threads_api_url).form(&params).send().await;

    match res {
        Ok(response) => {
            if response.status().is_success() {
                match response.json::<serde_json::Value>().await {
                    Ok(json) => {
                        info!("Media container created successfully. Response: {:?}", json);
                        if let Some(media_container_id) = json["id"].as_str() {
                            // Step 2: Publish the thread using the media container ID
                            let publish_url = format!(
                                "https://graph.threads.net/v1.0/{}/threads_publish",
                                threads_user_id
                            );
                            let publish_params = [
                                ("creation_id", media_container_id),
                                ("access_token", &access_token),
                            ];

                            let publish_res =
                                client.post(&publish_url).form(&publish_params).send().await;

                            match publish_res {
                                Ok(publish_response) => {
                                    if publish_response.status().is_success() {
                                        info!(
                                            "Post published successfully. Response: {:?}",
                                            publish_response.text().await.unwrap()
                                        );
                                    } else {
                                        error!(
                                            "Failed to publish post. Status code: {}, Response: {:?}",
                                            publish_response.status(),
                                            publish_response.text().await.unwrap()
                                        );
                                    }
                                }
                                Err(e) => {
                                    error!("Error publishing post: {}", e);
                                }
                            }
                        } else {
                            error!("Media container ID not found in response");
                        }
                    }
                    Err(e) => {
                        error!("Error parsing Threads API response JSON: {}", e);
                    }
                }
            } else {
                error!(
                    "Failed to create media container. Status code: {}, Response: {:?}",
                    response.status(),
                    response.text().await.unwrap()
                );
            }
        }
        Err(e) => {
            error!("Exception during posting: {}", e);
        }
    }
}