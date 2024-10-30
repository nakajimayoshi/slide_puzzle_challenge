#![allow(warnings)]
mod util;
mod test;
mod api;
mod tile;
mod traits;
mod puzzle;

use crate::api::submit_puzzle;
use crate::puzzle::Puzzle;
use crate::tile::Tile;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use redis::aio::MultiplexedConnection;
use redis::AsyncCommands;
use reqwest::Client;
use rustc_hash::FxHasher;
use serde::de::Error;
use serde::{Deserialize, Serialize};
use std::any::Any;
use std::cmp::{Ordering, PartialEq};
use std::fmt::Debug;
use std::fs::{self, File};
use std::future::Future;
use std::hash::{Hash, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::ops::Deref;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::sync::Semaphore;

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Ord, PartialOrd)]
#[repr(u8)]
enum Direction {
    UP,
    DOWN,
    LEFT,
    RIGHT
}

pub fn serialize_moves(moves: &Vec<Direction>) -> String {
    moves.iter().map(|d| d.to_char()).collect()
}

impl Direction {
    pub fn to_char(&self) -> char {
        match self {
            Direction::UP => 'U',
            Direction::DOWN => 'D',
            Direction::LEFT => 'L',
            Direction::RIGHT => 'R'
        }
    }

    pub fn inverse(&self) -> Direction {
        match self {
            Direction::UP => Direction::DOWN,
            Direction::DOWN => Direction::UP,
            Direction::LEFT => Direction::RIGHT,
            Direction::RIGHT => Direction::LEFT
        }
    }
}



fn hash_tiles(tiles: &Vec<Tile>) -> u64 {
    let mut hasher = FxHasher::default();

    for tile in tiles {
        tile.hash(&mut hasher);
    }

    hasher.finish()
}




type Matrix<T> = Vec<Vec<T>>;

fn tiles_vec_to_matrix(width: u32, height: u32, tiles: Vec<Tile>) -> Vec<Vec<Tile>> {
    let mut matrix: Vec<Vec<Tile>> = Vec::with_capacity(height as usize);
    let mut idx = 0;

    for _ in 0..height {
        let mut row: Vec<Tile> = Vec::with_capacity(width as usize);

        for _ in 0..width {
            if idx < tiles.len() {
                row.push(tiles[idx].clone());  // Add tile to the row, clone if necessary
                idx += 1;
            }
        }

        matrix.push(row); // Add the completed row to the matrix
    }

    matrix
}

impl PartialOrd for Tile {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for Tile { }

impl Ord for Tile {
    fn cmp(&self, other: &Self) -> Ordering {
        self.rank().cmp(&other.rank())
    }
}


async fn solve_single_threaded(client: &Client, pattern_db_client: &mut MultiplexedConnection) {

    const SLIDE_PUZZLE_COUNT: u32 = 10000;

    let bar = ProgressBar::new(SLIDE_PUZZLE_COUNT as u64);
    if let Ok(puzzles) = fs::File::open("slidepuzzle.txt") {

        let reader = BufReader::new(puzzles);

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let answers_file_name = format!("slide_puzzle_answers.txt_{}", timestamp);
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

                        if let Some(answer) = puzzle.solve(false) {
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

async fn solve_multithreaded(client: &Client, pattern_db_client: &MultiplexedConnection) {
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

        let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TASKS));
        let mut handles = Vec::new();

        for (idx, puzzle_str) in puzzle_strings.into_iter().enumerate() {
            let semaphore = semaphore.clone();
            let total_progress = Arc::clone(&total_progress);
            let puzzle_str = puzzle_str.clone();
            let puzzle_num = idx + 1;

            let handle = tokio::task::spawn(async move {
                let _permit = semaphore.acquire_owned().await.unwrap();

                // Create a new Redis connection for this task
                let redis_client = redis::Client::open("redis://localhost").unwrap();
                let mut conn = redis_client.get_multiplexed_tokio_connection().await.unwrap();

                match conn.get::<_, String>(&puzzle_str).await {
                    Ok(pattern) => {
                        total_progress.inc(1);
                        format!("{},{}\n", puzzle_num, pattern)
                    }
                    _ => {
                        let mut puzzle = Puzzle::from_str(&puzzle_str);
                        if let Some(moveset) = puzzle.solve(false) {
                            let moves_str = serialize_moves(&moveset);
                            let answer = format!("{},{}\n", puzzle_num, moves_str);
                            conn.set::<&str, &str, ()>(&puzzle_str, &moves_str)
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

// score: 30.21
#[tokio::main]
async fn main() {
    let reqwest_client = Client::new();

    const REDIS_HOST: &str = "localhost";
    const REDIS_PORT: u16 = 6379;
    let redis = redis::Client::open(format!("redis://{}:{}", REDIS_HOST, REDIS_PORT)).unwrap();
    let mut redis_con = redis.get_multiplexed_tokio_connection().await.unwrap();

    solve_multithreaded(&reqwest_client, &mut redis_con).await;
    // solve_single_threaded(&reqwest_client, &mut redis_con).await;

}