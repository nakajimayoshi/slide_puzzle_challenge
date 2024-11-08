use std::fs::File;
use std::io::{Read, Write};
use reqwest::{header, Client};
use reqwest::header::HeaderValue;
use reqwest::multipart;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SubmitBody {
    pub questions: Vec<u8>,
    pub answers: Vec<u8>
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct PuzzleSubmissionResponse {
    pub response_time: String,
    pub score: f32,
    pub limit_up: u32,
    pub limit_down: u32,
    pub limit_left: u32,
    pub limit_right: u32,
    pub count_up: u32,
    pub count_down: u32,
    pub count_left: u32,
    pub count_right: u32,
}

pub async fn get_slide_puzzle(client: &Client, puzzle_count: u32) {

    assert!(puzzle_count >= 10000);
    let url = format!("https://api.foresight.dev.metroweather.net/v1/recruitment/slidepuzzle/generate?count={}", puzzle_count);

    let mut headers = reqwest::header::HeaderMap::new();
    let accept = "*/*";

    let accept_header = HeaderValue::from_str(accept).unwrap();
    headers.insert("x-api-key", accept_header);


    let res = client.get(url)
        .headers(headers)
        .send()
        .await
        .unwrap();


    if res.status().is_success() {
        let body = res.text().await.unwrap();
        let mut file = File::create("slidepuzzle.txt").unwrap();

        match file.write_all(&body.as_bytes()) {
            Ok(_)  => {
                println!("saved puzzle to slidepuzzle.txt")
            },
            Err(e) => {
                println!("{:?}", e)
            }
        }

    } else {
        println!("Error: {:?}", res.status());
    }
}

pub async fn submit_puzzle(client: &Client, questions: &str, answers: &str) -> Result<PuzzleSubmissionResponse, Box<dyn std::error::Error>> {
    const URL: &'static str = "https://api.foresight.dev.metroweather.net/v1/recruitment/slidepuzzle";

    let form = multipart::Form::new()
        .file("questions", questions).await?
        .file("answers", answers).await?;

    let res = client.post(URL)
        .header("accept", "application/json")
        .multipart(form)
        .send()
        .await?
        .error_for_status()?;  // Automatically handle non-2xx statuses as errors

    let body = res.json::<PuzzleSubmissionResponse>().await?;
    Ok(body)
}
