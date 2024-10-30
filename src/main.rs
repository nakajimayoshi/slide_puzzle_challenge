#![allow(warnings)]
mod util;
mod test;
mod api;
mod tile;
mod traits;
mod puzzle;

use crate::api::submit_puzzle;
use crate::puzzle::{serialize_moves, Puzzle};
use crate::tile::Tile;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use redis::aio::MultiplexedConnection;
use redis::{AsyncCommands, Commands};
use reqwest::Client;
use rustc_hash::FxHasher;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::{Ordering, PartialEq};
use std::collections::VecDeque;
use std::fmt::Debug;
use std::fs::{self, File};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::ops::Deref;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::{Mutex, Semaphore};

async fn solve_single_threaded(client: &Client, pattern_db_client: &mut MultiplexedConnection) {

    const SLIDE_PUZZLE_COUNT: u32 = 10000;

    let bar = ProgressBar::new(SLIDE_PUZZLE_COUNT as u64);
    if let Ok(puzzles) = fs::File::open("slidepuzzle.txt") {

        let reader = BufReader::new(puzzles);

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let answers_file_name = format!("slide_puzzle_answers_{}.txt", timestamp);
        let mut answer_file = File::create(answers_file_name.clone()).unwrap();

        let mut line_idx: usize = 0;

        for line in reader.lines().skip(2) {

            let puzzle_num = line_idx + 1;
            match line {
                Ok(puzzle_str) => {

                    println!("{}", &puzzle_str);
                    if let Ok(answer) = pattern_db_client.get::<_, String>(&puzzle_str).await {
                            answer_file.write_all(format!("{},", puzzle_num).as_bytes()).unwrap();
                            answer_file.write_all(answer.as_bytes()).unwrap();
                            answer_file.write_all("\n".as_bytes()).unwrap();
                    } else {
                        let mut puzzle = Puzzle::from_str(&puzzle_str);

                        if let Some(answer) = puzzle.solve(false, 9.) {
                            let moves_str = serialize_moves(&answer);
                            answer_file.write_all(format!("{},", puzzle_num).as_bytes()).unwrap();
                            answer_file.write_all(moves_str.as_bytes()).unwrap();
                            answer_file.write_all("\n".as_bytes()).unwrap();
                        }
                    }
                },
                Err(e) => {
                    println!("error reading puzzle {:?}", e)
                }
            }

            bar.inc(1);

            line_idx+=1
        }

        let res = submit_puzzle(&client, "slidepuzzle.txt", &answers_file_name).await;

        match res {
            Ok(resp) => {
                println!("score: {}", resp.score);
            },
            Err(e) => {
                println!("{:?}", e);
            }
        }

    } else {

        api::get_slide_puzzle(&client, SLIDE_PUZZLE_COUNT).await;
    }
}

async fn solve_multithreaded(client: &Client, pattern_db_client: Arc<Mutex<MultiplexedConnection>>, heuristic_threshold: f32) {
    const SLIDE_PUZZLE_COUNT: u32 = 10000;
    const MAX_CONCURRENT_TASKS: usize = 32;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();

    if let Ok(puzzles) = fs::File::open("slidepuzzle.txt") {
        let file_content = {
            let mut content = String::new();
            let mut reader = BufReader::new(&puzzles);
            reader.read_to_string(&mut content).unwrap();
            content
        };

        let puzzle_strings: Vec<String> = file_content.lines().skip(2).map(String::from).collect();
        let total_progress = Arc::new(ProgressBar::new(puzzle_strings.len() as u64));
        total_progress.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap(),
        );

        let mut handles = Vec::new();

        for (idx, puzzle_str) in puzzle_strings.into_iter().enumerate() {
            let total_progress = Arc::clone(&total_progress);
            let puzzle_str = puzzle_str.clone();
            let puzzle_num = idx + 1;

            let pattern_db_client = Arc::clone(&pattern_db_client);

            let handle = tokio::task::spawn(async move {

                let mut pattern_db_client = pattern_db_client.lock().await;

                match pattern_db_client.get::<_, String>(&puzzle_str).await {
                    Ok(pattern) => {
                        total_progress.inc(1);
                        format!("{},{}\n", puzzle_num, pattern)
                    }
                    _ => {
                        let mut puzzle = Puzzle::from_str(&puzzle_str);
                        if let Some(moveset) = puzzle.solve(false, heuristic_threshold) {
                            let moves_str = serialize_moves(&moveset);
                            let answer = format!("{},{}\n", puzzle_num, moves_str);
                            pattern_db_client.set::<&str, &str, ()>(&puzzle_str, &moves_str)
                                .await
                                .unwrap();

                            total_progress.inc(1);
                            answer
                        } else {
                            total_progress.inc(1);
                            format!("{}\n", puzzle_num)
                        }
                    }
                }
            });

            handles.push(handle);
        }

        // Collect the answers
        let mut answers = Vec::new();

        for handle in handles {
            match handle.await {
                Ok(answer) => answers.push(answer),
                Err(e) => eprintln!("Task failed: {:?}", e),
            }
        }

        total_progress.finish_with_message("Completed processing all puzzles");

        // Write answers to file and submit...
        let answers_file_name = format!("slide_puzzle_answers.txt_{}", timestamp);
        let mut answer_file = File::create(&answers_file_name).unwrap();
        for answer in answers {
            answer_file.write_all(answer.as_bytes()).unwrap();
        }

        let res = submit_puzzle(&client, "slidepuzzle.txt", &answers_file_name).await;

        match res {
            Ok(resp) => {
                println!("score: {}", resp.score);
            }
            Err(e) => {
                println!("Submission failed: {:?}", e);
            }
        }
    } else {
        api::get_slide_puzzle(&client, SLIDE_PUZZLE_COUNT).await;
    }
}


fn ranges(start: f32, end: f32, step: f32) -> VecDeque<f32> {
    let mut values: VecDeque<f32> = VecDeque::with_capacity(((end - start) / step) as usize);
    let mut value = start;
    while value < end {
        values.push_back(value);
        value += step;
    }
    values
}

// score: 30.21
#[tokio::main(flavor = "multi_thread", worker_threads = 24)]
async fn main() {
    let reqwest_client = Client::new();

    const REDIS_HOST: &str = "localhost";
    const REDIS_PORT: u16 = 6379;
    let redis = redis::Client::open(format!("redis://{}:{}", REDIS_HOST, REDIS_PORT)).unwrap();
    // let mut redis_con = Arc::new(tokio::sync::Mutex::new(redis.get_multiplexed_tokio_connection().await.unwrap()));

    let mut redis_con = redis.get_connection().unwrap();

    let save_file = "backups/answers_backup.txt";

    let mut file = File::create(save_file).unwrap();

    let keys: Vec<String> = redis_con.keys("*").unwrap();

    for key in keys {
        let key: String = key;
        let value: String = redis_con.get(&key).unwrap();
        file.write_all(format!("{}:{}\n", key, value).as_bytes()).unwrap();
    }




    // let mut heuristic_thresholds = ranges(39., 40., 0.1);
    //
    // while let (Some(front), Some(back)) = (heuristic_thresholds.pop_front(), heuristic_thresholds.pop_back()) {
    //     println!("solving with threshold: {}," , front);
    //     solve_multithreaded(&reqwest_client, Arc::clone(&redis_con), front).await;
    //     println!("solving with threshold: {}," , back);
    //     solve_multithreaded(&reqwest_client, Arc::clone(&redis_con), back).await;
    // }
    // solve_single_threaded(&reqwest_client, &mut redis_con).await;

}