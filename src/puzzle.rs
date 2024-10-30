use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::thread::sleep;
use rustc_hash::{FxHashSet, FxHasher};
use crate::{hash_tiles, Direction};
use crate::tile::Rune::{SPACE, WALL};
use crate::tile::Tile;
use crate::traits::puzzle::{DebugPrintable, Heuristic};


#[derive(Debug)]
pub enum PuzzleError {
    IllegalMove(String),
    UnsolvableBoard(String)
}

impl fmt::Display for PuzzleError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            PuzzleError::IllegalMove(msg) => write!(f, "Illegal move: {}", msg),
            PuzzleError::UnsolvableBoard(msg) => write!(f, "Board is in an unsolvable configuration {}", msg)
        }
    }
}


impl std::error::Error for PuzzleError {}


pub enum PuzzleRouteDirection {
    Forward,
    Reverse
}

pub struct PuzzleRoute {
    state_hash: u64,
    moves: Vec<Direction>,
    direction: PuzzleRouteDirection
}

#[derive(Clone, Debug, Eq, Ord, PartialOrd, PartialEq, Hash)]
pub struct Puzzle {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) tiles: Vec<Tile>,
    moves: Vec<Direction>,
    hash: u64,
    g: u32,
}

const RADIX: u32 = 10;
impl Puzzle {
    pub fn from_str(str: &str) -> Self {

        let width= str.chars().nth(0).unwrap().to_digit(RADIX).unwrap();
        let height= str.chars().nth(2).unwrap().to_digit(RADIX).unwrap();

        let mut tiles: Vec<Tile> = Vec::with_capacity((width * height) as usize);

        for char in str.chars().skip(4) {

            let tile = Tile::new(char);
            tiles.push(tile);
        }


        let hash = hash_tiles(&tiles);

        Self {
            tiles,
            width,
            height,
            moves: vec![],
            hash,
            g: 0,
        }
    }

    fn inverse_manhattan_distance(&self, tile: &Tile, root_puzzle: &Puzzle) -> u32 {

        if self.hash == root_puzzle.hash {
            return 0
        }

        if tile.rune == WALL {
            return 0;  // Walls themselves have no "goal" distance
        }

        let idx = self.tiles.iter().position(|t| t.raw == tile.raw).unwrap();
        let root_idx = root_puzzle.tiles.iter().position(|t| t.raw == tile.raw).unwrap();
        let current_row = idx as u32 / self.width;
        let current_col = idx as u32 % self.width;
        let original_row = root_idx as u32 / self.width;
        let original_col = root_idx as u32 % self.width;

        let mut lateral_moves = (current_col as i32 - original_col as i32).abs() as u32;
        let mut vertical_moves = (current_row as i32 - original_row as i32).abs() as u32;

        if current_col != original_col {
            let step = if original_col > current_col { 1 } else { - 1 };
            for col in (current_col as i32..original_col as i32).step_by(step as usize) {
                let check_idx = (current_row * self.width + col as u32) as usize;
                if self.tiles[check_idx].rune == WALL {
                    lateral_moves += 2
                }
            }
        }

        if current_row != original_row {
            let step = if original_row > current_row { 1 } else { -1 };

            for row in (current_row as i32..original_row as i32).step_by(step as usize) {
                let check_idx = (row as u32 * self.width + current_col) as usize;
                if self.tiles[check_idx].rune == WALL {
                    vertical_moves += 2;
                }
            }
        }


        lateral_moves + vertical_moves
    }

    fn get_inverse_heuristic(&self, root_puzzle: &Puzzle) -> u32 {
        let mut score: u32 = 0;

        for tile in &self.tiles {
            score+=self.inverse_manhattan_distance(tile, root_puzzle);
        }

        score
    }

    pub(crate) fn manhattan_distance(&self, tile: &Tile, solved_puzzle: &Puzzle) -> u32 {
        if tile.rune == WALL {
            return 0;  // Walls themselves have no "goal" distance
        }

        let idx = self.tiles.iter().position(|t| t.raw == tile.raw).unwrap();

        let mut solved_idx: usize = 0;

        if let Some(tile_solved_idx) = tile.solved_idx {
            solved_idx = tile_solved_idx;
        } else {
            solved_idx = solved_puzzle.tiles.iter().position(|t| t.rank() == tile.rank()).unwrap();
        }

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
                    vertical_moves += 2;
                }
            }
        }

        lateral_moves + vertical_moves
    }

    fn linear_conflicts(&self, tile_idx: usize) -> u32 {
        let tile = self.tiles.get(tile_idx).unwrap();

        let mut linear_conflicts = 0;

        if let Some(right) = self.tiles.get(tile_idx + 1) {
            let is_reversal = (tile.rank() - right.rank()).abs() == 1;
            if is_reversal {
                linear_conflicts+=1;
            }
        }

        if tile_idx >= 1 {
            if let Some(left) = self.tiles.get(tile_idx - 1) {
                let is_reversal = (tile.rank() - left.rank()).abs() == 1;
                if is_reversal {
                    linear_conflicts+=1;
                }
            }
        }

        if tile_idx >= self.width as usize {
            if let Some(up) = self.tiles.get(tile_idx - self.width as usize) {
                let is_reversal = (tile.rank() - up.rank()).abs() == 1;
                if is_reversal {
                    linear_conflicts+=1;
                }
            }
        }



        if let Some(down) = self.tiles.get(tile_idx + self.width as usize) {
            let is_reversal = (tile.rank() - down.rank()).abs() == 1;

            if is_reversal {
                linear_conflicts+=1;
            }
        }

        linear_conflicts
    }

    fn inversions(&self) -> u32 {
        let mut inversions = 0;

        for (idx, tile) in self.tiles.iter().enumerate() {
            let forward_tiles = &self.tiles[idx+1..self.tiles.len()];

            for forward_tile in forward_tiles {
                if forward_tile.rank() < tile.rank() {
                    inversions+=1;
                }
            }
        }

        inversions
    }

    fn is_solved(&self, solved_puzzle: &Puzzle) -> bool {
        self.hash == solved_puzzle.hash
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
            },
        }

        self.hash = hash_tiles(&self.tiles);

        Ok(())
    }

    pub fn moves_str(&self) -> String {
        self.moves.iter().map(|d| d.to_char()).collect()
    }

    fn generate_successors(&self, space_idx: usize) -> Vec<Puzzle> {
        let mut result: Vec<Puzzle> = Vec::with_capacity(4);

        for move_ in self.legal_moves(space_idx) {
            let mut successor = self.clone();
            successor.move_space(move_).unwrap();
            let mut hasher = FxHasher::default();
            let new_hash = {
                for tile in &successor.tiles {
                    tile.hash(&mut hasher);
                }
                hasher.finish()
            };
            successor.hash = new_hash;
            result.push(successor)
        }

        result
    }

    fn is_solvable(&self) -> bool {
        // let
        // let inversions = self.inversions();
        // if inversions % 2 != 0 {
        //     println!("Puzzle is unsolvable: {}", inversions)
        // }
        true
    }

    pub fn solve(&mut self, debug: bool) -> Option<Vec<Direction>> {

        const STEP: u32 = 1;
        // const MAX_ITERATIONS: u32 = 100;
        let solved_puzzle = self.solved();

        let mut open_list = BinaryHeap::<(Reverse<u32>, Puzzle)>::new();
        let mut closed_list = FxHashSet::default();

        let space_idx = self.space_idx();

        for neighbour in self.generate_successors(space_idx) {
            let new_cost = self.g + STEP;
            let heuristic = new_cost + neighbour.get_heuristic(&solved_puzzle);

            // Return the heuristic, neighbor, and new path
            open_list.push((Reverse(heuristic), neighbour));
        }

        // let mut iteration: u32 = 0;
        while let Some((Reverse(_heuristic), puzzle)) = open_list.pop() {

            // iteration += 1;
            // if iteration > MAX_ITERATIONS {
            //     break;
            // }

            if debug {
                puzzle.debug_print(false);
                println!("states visited: {}", closed_list.len());
                println!("Heuristic: {}", puzzle.get_heuristic(&solved_puzzle));
            }

            if puzzle.is_solved(&solved_puzzle) {
                *self = puzzle.clone();
                return Some(self.moves.to_vec())
            }


            if closed_list.insert(puzzle.hash)  {
                let space_idx = puzzle.space_idx();
                for neighbour in puzzle.generate_successors(space_idx) {
                    let new_cost = puzzle.g + STEP;
                    let heuristic = new_cost + neighbour.get_heuristic(&solved_puzzle);

                    open_list.push((Reverse(heuristic), neighbour));
                }
            }

            if debug {
                sleep(std::time::Duration::from_millis(5));
            }
        }
        None
    }

    fn serialized(&self) -> String {
        let tiles_str: String = self.tiles.iter().map(|t| t.raw).collect();

        format!("{},{},{}", self.width, self.height, tiles_str)
    }

    fn space_idx(&self) -> usize {
        self.tiles.iter().position(|t| { t.rune == SPACE }).unwrap()
    }

    fn legal_moves(&self, space_idx: usize) -> Vec<Direction> {
        let mut legal_moves: Vec<Direction> = Vec::with_capacity(4);

        legal_moves.push(Direction::UP);
        legal_moves.push(Direction::DOWN);
        legal_moves.push(Direction::LEFT);
        legal_moves.push(Direction::RIGHT);

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


    pub(crate) fn solved(&self) -> Puzzle {
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


        let hash = hash_tiles(&solved_tiles);

        // Construct the solved puzzle
        Puzzle {
            width: self.width,
            height: self.height,
            tiles: solved_tiles,
            moves: vec![],
            hash,
            g: 0,
        }
    }

}