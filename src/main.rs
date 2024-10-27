#![allow(warnings)]
mod util;
mod test;
mod api;

use crate::Rune::{SPACE, VALUE, WALL};
use reqwest::header::HeaderValue;
use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering::{Equal, Greater, Less};
use std::cmp::{Ordering, PartialEq, Reverse};
use std::fmt::{write, Debug};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Write};
use std::ops::Deref;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::env::current_exe;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread::{current, sleep};
use chrono::format;
use colored::Colorize;
use crossbeam::atomic::AtomicCell;
use futures::stream::FuturesUnordered;
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rayon::prelude::IntoParallelIterator;
use rayon::prelude::*; // Import the ParallelIterator trait and others
use serde::de::Error;
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::sync::mpsc;
use tokio::task;
use uuid::Uuid;
use crate::api::{submit_puzzle, PuzzleSubmissionResponse, SubmitBody};




#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq, Ord, PartialOrd)]
#[repr(u8)]
enum Direction {
    UP,
    DOWN,
    LEFT,
    RIGHT
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
}

#[derive(Debug)]
pub enum PuzzleError {
    IllegalMove(String),
}

impl fmt::Display for PuzzleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PuzzleError::IllegalMove(msg) => write!(f, "Illegal move: {}", msg),
        }
    }
}

impl std::error::Error for PuzzleError {}

#[derive(Clone, Debug, Hash, PartialEq, Eq, Ord, PartialOrd)]
struct Puzzle {
    width: u32,
    height: u32,
    tiles: Vec<Tile>,
    moves: Vec<Direction>,
}
const RADIX: u32 = 10;
impl Puzzle {
    pub fn from_str(str: String) -> Self {

        let width= str.chars().nth(0).unwrap().to_digit(RADIX).unwrap();
        let height= str.chars().nth(2).unwrap().to_digit(RADIX).unwrap();

        let mut tiles: Vec<Tile> = Vec::with_capacity((width * height) as usize);

        for char in str.chars().skip(4) {

            let tile = Tile::new(char);
            tiles.push(tile);
        }

        Self {
            tiles,
            width,
            height,
            moves: vec![],
        }
    }

    fn manhattan_distance(&self, tile: &Tile) -> u32 {
        if tile.rune == WALL {
            return 0;  // Walls themselves have no "goal" distance
        }

        let idx = self.tiles.iter().position(|t| t.raw == tile.raw).unwrap();
        let solved_puzzle = self.solved();
        let solved_idx = solved_puzzle.tiles.iter().position(|t| t.raw == tile.raw).unwrap();
        let current_row = idx as u32 / self.width;
        let current_col = idx as u32 % self.width;
        let solved_row = solved_idx as u32 / self.width;
        let solved_col = solved_idx as u32 % self.width;

        let mut lateral_moves = (current_col as i32 - solved_col as i32).abs() as u32;
        let mut vertical_moves = (current_row as i32 - solved_row as i32).abs() as u32;

        if current_col != solved_col {
            let step = if solved_col > current_col { 1 } else { -1 };
            for col in (current_col as i32..solved_col as i32).step_by(step as usize) {
                let check_idx = (current_row * self.width + col as u32) as usize;
                if self.tiles[check_idx].rune == WALL {
                    lateral_moves += 2;
                }
            }
        }

        if current_row != solved_row {
            let step = if solved_row > current_row { 1 } else { -1 };
            for row in (current_row as i32..solved_row as i32).step_by(step as usize) {
                let check_idx = (row as u32 * self.width + current_col) as usize;
                if self.tiles[check_idx].rune == WALL {
                    vertical_moves += 2; // Add a penalty if a wall blocks the path
                }
            }
        }

        lateral_moves + vertical_moves
    }

    fn is_solved(&self) -> bool {
        self.tiles == self.solved().tiles
    }

    fn debug_print(&self) {
        print!("┌");
        for col in 0..self.width {
            print!("───");
            if col < self.width - 1 {
                print!("┬");
            }
        }
        println!("┐");

        for row in 0..self.height {
            print!("│");
            for col in 0..self.width {
                let idx = (row * self.width + col) as usize;
                if let Some(tile) = self.tiles.get(idx) {
                    match tile.rune {
                        WALL => print!(" {} ", "█"),
                        SPACE => print!(" {} ", " ".green()),
                        _ => print!(" {} ", tile.raw),
                    }
                }
                print!("│");
            }
            println!();

            if row < self.height - 1 {
                print!("├");
                for col in 0..self.width {
                    print!("───");
                    if col < self.width - 1 {
                        print!("┼");
                    }
                }
                println!("┤");
            }
        }

        print!("└");
        for col in 0..self.width {
            print!("───");
            if col < self.width - 1 {
                print!("┴");
            }
        }
        println!("┘");
    }

    fn move_space(&mut self, dir: Direction) -> Result<(), PuzzleError> {
        let space_idx = self.space_idx();

        match dir {
            Direction::UP => {
                let target_idx = space_idx.checked_sub(self.width as usize)
                    .ok_or(PuzzleError::IllegalMove("Cannot move up from top edge".into()))?;

                if self.tiles[target_idx].rune == WALL {
                    return Err(PuzzleError::IllegalMove("Cannot move space up".into()));
                }
                self.tiles.swap(space_idx, target_idx);
                self.moves.push(Direction::UP);
                Ok(())
            },
            Direction::DOWN => {
                let target_idx = space_idx + self.width as usize;
                if target_idx >= self.tiles.len() {
                    return Err(PuzzleError::IllegalMove("Cannot move down from bottom edge".into()));
                }

                if self.tiles[target_idx].rune == WALL {
                    return Err(PuzzleError::IllegalMove("Cannot move space down".into()));
                }
                self.tiles.swap(space_idx, target_idx);
                self.moves.push(Direction::DOWN);
                Ok(())
            },
            Direction::LEFT => {
                let target_idx = space_idx.checked_sub(1)
                    .ok_or(PuzzleError::IllegalMove("Cannot move left from left edge".into()))?;

                if space_idx % self.width as usize == 0 {
                    return Err(PuzzleError::IllegalMove("Cannot move left from left edge".into()));
                }
                if self.tiles[target_idx].rune == WALL {
                    return Err(PuzzleError::IllegalMove("Cannot move space left".into()));
                }
                self.tiles.swap(space_idx, target_idx);
                self.moves.push(Direction::LEFT);
                Ok(())
            },
            Direction::RIGHT => {
                let target_idx = space_idx + 1;
                if target_idx >= self.tiles.len() || (space_idx + 1) % self.width as usize == 0 {
                    return Err(PuzzleError::IllegalMove("Cannot move right from right edge".into()));
                }
                if self.tiles[target_idx].rune == WALL {
                    return Err(PuzzleError::IllegalMove("Cannot move space right".into()));
                }
                self.tiles.swap(space_idx, target_idx);
                self.moves.push(Direction::RIGHT);
                Ok(())
            },
        }
    }

    pub fn moves_str(&self) -> String {
        self.moves.iter().map(|d| d.to_char()).collect()
    }

    fn generate_successors(&self) -> Vec<(Puzzle, Direction)> {
        let mut result: Vec<(Puzzle, Direction)> = vec![];

        for move_ in self.legal_moves() {
            let mut successor = self.clone();
            successor.move_space(move_).unwrap();
            result.push((successor, move_));
        }

        result
    }


    pub fn solve(&mut self) -> Option<Vec<Direction>> {

        let mut open_list = BinaryHeap::new();
        let mut closed_list = HashSet::new(); // Changed from HashMap to HashSet
        open_list.push((Reverse(self.get_heuristic()), 0, self.clone(), vec![]));

        // Add a maximum iteration limit as a safeguard
        let mut iterations = 0;
        const MAX_ITERATIONS: usize = 1000;

        while let Some((Reverse(_heuristic), g, puzzle, path)) = open_list.pop() {
            // Add iteration limit check
            iterations += 1;
            if iterations > MAX_ITERATIONS {
                return None;
            }

            if puzzle.is_solved() {
                *self = puzzle.clone();
                self.moves = path.clone();
                return Some(path);
            }

            // Only explore this state if we haven't seen it before
            if closed_list.insert(puzzle.clone()) {
                for (neighbor, direction) in puzzle.generate_successors() {
                    let new_cost = g + 1;
                    let heuristic = new_cost + neighbor.get_heuristic();

                    // Simplified check - we rely on HashSet for visited states
                    let mut new_path = path.clone();
                    new_path.push(direction);
                    open_list.push((Reverse(heuristic), new_cost, neighbor, new_path));
                }
            }
        }

        None
    }

    pub fn solve_parallel(&mut self) -> Option<Vec<Direction>> {
        const NUM_WORKERS: usize = 24;

        let best_solution = Arc::new(AtomicCell::new(None));

        // Shared priority queue for open lists
        let open_lists: Vec<Mutex<BinaryHeap<_>>> = (0..NUM_WORKERS).map(|_| Mutex::new(BinaryHeap::new())).collect();

        // Initial setup - generate initial paths and assign to open lists
        for (i, (neighbor, direction)) in self.generate_successors().into_iter().enumerate() {
            let mut initial_path = vec![direction];
            let heuristic = neighbor.get_heuristic();
            open_lists[i % NUM_WORKERS as usize].lock().unwrap().push((Reverse(heuristic), 1, neighbor, initial_path));
        }

        (0..NUM_WORKERS).into_par_iter().for_each(|worker_id| {
            let open_list = &open_lists[worker_id];
            let mut closed_list = HashSet::new();

            while let Some((Reverse(_heuristic), g, puzzle, path)) = open_list.lock().unwrap().pop() {
                if puzzle.is_solved() {
                    // Atomically update the best solution found
                    let solution = Some(path.clone());
                    best_solution.swap(solution);
                    return; // Stop this thread since a solution is found
                }

                // Only explore this state if it hasn't been seen in this thread
                if closed_list.insert(puzzle.clone()) {
                    for (neighbor, direction) in puzzle.generate_successors() {
                        let new_cost = g + 1;
                        let heuristic = new_cost + neighbor.get_heuristic();

                        let mut new_path = path.clone();
                        new_path.push(direction);
                        open_list.lock().unwrap().push((Reverse(heuristic), new_cost, neighbor, new_path));
                    }
                }

                // Check if another thread has found a solution
                if best_solution.take().is_some() {
                    break; // Exit the loop if a solution is already found
                }
            }
        });

        // Return the best solution found (if any)
        best_solution.take()
    }

    pub fn serialized(&self) -> String {
        self.tiles.clone().into_iter().map(|t| { t.raw }).collect()
    }

    fn space_idx(&self) -> usize {
       self.tiles.iter().position(|t| { t.rune == SPACE }).unwrap()
    }

    fn generate_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn legal_moves(&self) -> Vec<Direction> {
        let mut legal_moves: Vec<Direction> = vec![Direction::UP, Direction::DOWN, Direction::LEFT, Direction::RIGHT];

        let space_idx = self.space_idx();
        let row = space_idx / self.width as usize;
        let col = space_idx % self.width as usize;

        // Boundary checks
        if row == 0 { // Top row
            legal_moves.retain(|&d| d != Direction::UP);
        }
        if row == (self.height - 1) as usize { // Bottom row
            legal_moves.retain(|&d| d != Direction::DOWN);
        }
        if col == 0 { // Left edge
            legal_moves.retain(|&d| d != Direction::LEFT);
        }
        if col == (self.width - 1) as usize { // Right edge
            legal_moves.retain(|&d| d != Direction::RIGHT);
        }

        // Wall checks
        if let Some(tile) = self.tiles.get(space_idx + 1) {
            if tile.rune == WALL {
                legal_moves.retain(|&d| d != Direction::RIGHT);
            }
        }
        if space_idx >= 1 {
            if let Some(tile) = self.tiles.get(space_idx - 1) {
                if tile.rune == WALL {
                    legal_moves.retain(|&d| d != Direction::LEFT);
                }
            }
        }
        if let Some(tile) = self.tiles.get(space_idx + self.width as usize) {
            if tile.rune == WALL {
                legal_moves.retain(|&d| d != Direction::DOWN);
            }
        }
        if space_idx >= self.width as usize {
            if let Some(tile) = self.tiles.get(space_idx - self.width as usize) {
                if tile.rune == WALL {
                    legal_moves.retain(|&d| d != Direction::UP);
                }
            }
        }

        legal_moves
    }

    fn solved(&self) -> Puzzle {
        let mut solved_tiles = self.tiles.clone();

        // Remove any wall tiles and blank spaces, sort the rest
        solved_tiles.retain(|tile| tile.raw != '0' && tile.raw != '=');
        solved_tiles.sort_by(|a, b| a.rank().cmp(&b.rank()));

        // Add the blank tile ('0') at the end
        solved_tiles.push(Tile::new('0'));

        // Reinsert wall tiles ('=') in their original positions
        for (idx, tile) in self.tiles.iter().enumerate() {
            if tile.raw == '=' {
                solved_tiles.insert(idx, Tile::new('='));
            }
        }

        // Construct the solved puzzle
        Puzzle {
            width: self.width,
            height: self.height,
            tiles: solved_tiles,
            moves: vec![],
        }
    }


}

pub trait Heuristic {
    fn get_heuristic(&self) -> u32;
}

impl Heuristic for Puzzle {
    fn get_heuristic(&self) -> u32 {
        let mut score: u32 = 0;

        for tile in &self.tiles {
            score+=self.manhattan_distance(tile);
        }

        score
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Hash)]
enum Rune {
    VALUE,
    SPACE,
    WALL
}

impl Rune {
    pub fn from_char(char: char) -> Self {
        match char {
            '=' => WALL,
            '0' => SPACE,
            _ => VALUE
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq)]
struct Tile {
    raw: char,
    rune: Rune,
}


impl Tile {
    pub fn new(char: char) -> Self {

        Self {
            raw: char,
            rune: Rune::from_char(char),
        }
    }

    pub fn raw(&self) -> char {
        self.raw
    }

    pub fn rank(&self) -> i32 {
        match self.raw {
            '1'..='9' => self.raw as i32 - '0' as i32,
            'a'..='z' => self.raw as i32 - 'a' as i32 + 10,
            'A'..='Z' => self.raw as i32 - 'A' as i32 + 36,
            '0' => 62,
            '=' => -1,
            _ => -1,
        }
    }
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


async fn solve_single_threaded(client: &Client) {

    const SLIDE_PUZZLE_COUNT: u32 = 10000;

    let bar = ProgressBar::new(SLIDE_PUZZLE_COUNT as u64);
    if let Ok(puzzles) = fs::File::open("slidepuzzle.txt") {

        let reader = BufReader::new(puzzles);

        let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();
        let answers_file_name = format!("slide_puzzle_answers.txt_{}", timestamp);
        let mut answer_file = File::create(answers_file_name.clone()).unwrap();

        let mut line_idx = 0;

        for line in reader.lines().skip(2) {

            let puzzle_num = line_idx + 1;
            match line {
                Ok(puzzle_str) => {
                    let mut puzzle = Puzzle::from_str(puzzle_str);

                    puzzle.solve();

                    if puzzle.is_solved() {
                        let moves = puzzle.moves_str();
                        // write puzzle_mum then comma
                        answer_file.write_all(format!("{},", puzzle_num).as_bytes()).unwrap();
                        answer_file.write_all(moves.as_bytes()).unwrap();
                        answer_file.write_all("\n".as_bytes()).unwrap();
                    } else {
                        answer_file.write_all("UNSOLVABLE\n".as_bytes()).unwrap();
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
async fn solve_multithreaded(client: &Client) {
    use rayon::prelude::*; // Use Rayon for better CPU utilization

    const SLIDE_PUZZLE_COUNT: u32 = 10000;
    const CHUNK_SIZE: usize = 100;

    let timestamp = chrono::Utc::now().format("%Y-%m-%dT%H-%M-%S").to_string();

    if let Ok(puzzles) = fs::File::open("slidepuzzle.txt") {
        let file_content = {
            let mut content = String::new();
            let mut reader = BufReader::new(&puzzles);
            reader.read_to_string(&mut content).unwrap();
            content
        };

        let puzzle_strings: Vec<String> = file_content.lines().skip(2).map(String::from).collect();
        let multi_progress = Arc::new(MultiProgress::new());
        let total_progress = multi_progress.add(ProgressBar::new(puzzle_strings.len() as u64));
        total_progress.set_style(
            ProgressStyle::default_bar()
                .template("[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}")
                .unwrap(),
        );

        let answers: Vec<String> = puzzle_strings
            .par_iter()
            .enumerate()
            .map(|(idx, puzzle_str)| {
                let puzzle_num = idx + 1;
                let mut puzzle = Puzzle::from_str(puzzle_str.to_string());
                puzzle.solve();

                let result = if puzzle.is_solved() {
                    format!("{},{}\n", puzzle_num, puzzle.moves_str())
                } else {
                    format!("{},UNSOLVABLE\n", puzzle_num)
                };

                total_progress.inc(1);
                result
            })
            .collect();

        total_progress.finish_with_message("Completed processing all puzzles");

        let answer_file = File::create(format!("slidepuzzle_answers_{}.txt", timestamp)).unwrap();
        let mut writer = std::io::BufWriter::new(answer_file);

        for answer in answers.iter() {
            writer.write_all(answer.as_bytes()).unwrap();
        }

        let res = submit_puzzle(&client, "slidepuzzle.txt", format!("slidepuzzle_answers_{}.txt", timestamp).as_str()).await;

        match res {
            Ok(resp) => {
                println!("score: {}", resp.score);
            }
            Err(e) => {
                println!("{:?}", e);
            }
        }
    } else {
        api::get_slide_puzzle(&client, SLIDE_PUZZLE_COUNT).await;
    }
}





#[tokio::main]
async fn main() {
    let client = Client::new();
    // solve_single_threaded(&client).await;
    solve_multithreaded(&client).await;

    // let res = submit_puzzle(&client, "slidepuzzle.txt", "slide_puzzle_answers.txt_2024-10-27T09-08-07").await;
    //
    // match res {
    //     Ok(resp) => {
    //         println!("score: {}", resp.score);
    //     },
    //     Err(e) => {
    //         println!("{:?}", e);
    //     }
    // }
}