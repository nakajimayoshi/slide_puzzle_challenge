mod util;
mod test;

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
use std::thread::{current, sleep};
use colored::Colorize;
use serde::de::Error;

async fn get_slide_puzzle(client: &reqwest::Client, puzzle_count: u32) {
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

#[derive(Deserialize, Serialize, Debug, Clone)]
struct SubmitBody {
    questions: Vec<u8>,
    answers: Vec<u8>
}


#[derive(Deserialize, Serialize, Debug, Clone)]
struct PuzzleSubmissionResponse {
    response_time: String,
    score: u64,
    limit_up: u64,
    limit_down: u64,
    limit_left: u64,
    limit_right: u64,
    count_up: u64,
    count_down: u64,
    count_left: u64,
    count_right: u64
}

fn get_file_as_byte_vec(filename: &str) -> Vec<u8> {

    let mut f = File::open(filename).unwrap();
    let mut buffer = Vec::new();

    f.read_to_end(&mut buffer).unwrap();
    buffer
}

async fn submit_puzzle(client: &Client, questions: &str, answers: &str) {
    const URL: &'static str = "https://api.foresight.dev.metroweather.net/v1/recruitment/slidepuzzle";

    let mut headers = reqwest::header::HeaderMap::new();
    let accept_header = header::HeaderValue::from_str("application/json");
    let content_type_header = header::HeaderValue::from_str("multipart/form-data");

    headers.insert("accept", accept_header.unwrap());
    headers.insert("Content-Type", content_type_header.unwrap());


    let body = SubmitBody {
        questions: get_file_as_byte_vec("slizepuzzle.txt"),
        answers: get_file_as_byte_vec("slizepuzzle_answers.txt")
    };


    let res = client.post(URL)
        .headers(headers)
        .body(serde_json::to_string(&body).unwrap())
        .send()
        .await
        .unwrap();

    if res.status().is_success() {
        let resp_body = res.text().await.unwrap();

        let resp: PuzzleSubmissionResponse = serde_json::from_str(&resp_body).unwrap();

        println!("{:?}", resp);

    }
}


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


        println!("puzzle string: {:?}", str);
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
        self.generate_hash() == self.solved().generate_hash()
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
        self.moves.iter().map( |d| { d.to_char() }).collect()
    }

    fn generate_successors(&self) -> (Vec<(Puzzle, Direction)>) {
        let mut result: Vec<(Puzzle, Direction)> = vec![];

        for move_ in self.legal_moves() {
            let mut successor = self.clone();
            successor.move_space(move_).unwrap();
            result.push((successor, move_));
        }

        result
    }

    fn is_solvable(&self) -> bool {
        let width = self.width as usize;
        let height = self.height as usize;

        // Flatten the tile board into a single vector, excluding the blank space
        let tiles: Vec<u32> = self
            .tiles
            .iter()
            .filter(|&tile| tile.raw != '0')// Exclude blank tile represented by '0'
            .map(|tile| tile.rank() as u32)
            .collect();

        // Count inversions
        let inversions = Self::count_inversions(&tiles);

        if width % 2 == 1 {
            // Odd-width grid: solvable if inversions are even
            inversions % 2 == 0
        } else {
            // Even-width grid: consider row position of the blank space
            let space_idx = self.space_idx();
            let space_row_from_bottom = height - (space_idx / width);

            (inversions % 2 == 0 && space_row_from_bottom % 2 == 1)
                || (inversions % 2 == 1 && space_row_from_bottom % 2 == 0)
        }
    }

    // Helper function to count inversions in the tile array
    fn count_inversions(tiles: &[u32]) -> u32 {
        let mut inversions = 0;
        for i in 0..tiles.len() {
            for j in (i + 1)..tiles.len() {
                if tiles[i] > tiles[j] {
                    inversions += 1;
                }
            }
        }
        inversions
    }

    pub fn solve(&mut self) -> Option<Vec<Direction>> {
        if !self.is_solvable() {
            println!("Puzzle is unsolvable.");
            return None;
        }

        let mut open_list = BinaryHeap::new();
        let mut closed_list = HashMap::<Puzzle, u32>::new();
        open_list.push((Reverse(self.get_heuristic()), 0, self.clone(), vec![]));

        while let Some((Reverse(_heuristic), g, puzzle, path)) = open_list.pop() {
            puzzle.debug_print();
            if puzzle.is_solved() {
                println!("Solved in {} moves", path.len());
                *self = puzzle.clone();
                self.moves = path.clone();
                return Some(path);
            }

            // Insert into closed_list only if this path is cheaper than any previous path to this state
            if closed_list.get(&puzzle).map_or(true, |&cost| g < cost) {
                closed_list.insert(puzzle.clone(), g);

                for (neighbor, direction) in puzzle.generate_successors() {
                    let new_cost = g + 1;
                    let heuristic = new_cost + neighbor.get_heuristic();

                    if closed_list.get(&neighbor).map_or(true, |&cost| new_cost < cost) {
                        let mut new_path = path.clone();
                        new_path.push(direction);
                        open_list.push((Reverse(heuristic), new_cost, neighbor, new_path));
                    }
                }
            }

            println!("hash: {}", puzzle.generate_hash());
            sleep(std::time::Duration::from_millis(500));

        }

        None
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
        let mut puzzle_answer = self.clone();

        let mut wall_idxs: Vec<usize> = vec![];

        let mut idx = 0;
        for tile in &puzzle_answer.tiles {
            if tile.rune == WALL {
                wall_idxs.push(idx)
            }
            idx+=1
        }

        puzzle_answer.tiles.sort_unstable();
        puzzle_answer.tiles.retain(|t| t.rune != SPACE);
        puzzle_answer.tiles.push(Tile::new('0'));
        puzzle_answer.tiles.retain(|t| t.rune != WALL);

        for idx in wall_idxs {
            puzzle_answer.tiles.insert(idx, Tile::new('='));
        }

        puzzle_answer
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
            'A'..='Z' => self.raw as i32 - 'A' as i32 + 10,
            'a'..='z' => self.raw as i32 - 'a' as i32 + 36,
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


#[tokio::main]
async fn main() {

    let client = Client::new();

    if let Ok(puzzles) = fs::File::open("slidepuzzle.txt") {

        let reader = BufReader::new(puzzles);

        let mut line_idx = 0;

        for line in reader.lines().skip(2) {

            match line {
                Ok(puzzle_str) => {
                    let mut puzzle = Puzzle::from_str(puzzle_str);

                    puzzle.solve();

                    puzzle.debug_print();
                },
                Err(e) => {
                    println!("error reading puzzle {:?}", e)
                }
            }

            line_idx+=1
        }

        // submit_puzzle(&client, "slidepuzzle.txt", "slizepuzzle_answers.txt").await;

    } else {
        const SLIDE_PUZZLE_COUNT: u32 = 10000;
        get_slide_puzzle(&client, SLIDE_PUZZLE_COUNT).await;
    }
}
