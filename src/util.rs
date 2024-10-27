use std::fs;
use std::io::{BufRead, BufReader};
use crate::Puzzle;

pub fn read_puzzles() -> Vec<Puzzle> {
    let mut result: Vec<Puzzle> = vec![];
    if let Ok(puzzles) = fs::File::open("slidepuzzle.txt") {
        let reader = BufReader::new(puzzles);

        let mut line_idx = 0;

        for line in reader.lines().skip(1) {
            match line {
                Ok(puzzle_str) => {
                    let puzzle = Puzzle::from_str(puzzle_str);
                    result.push(puzzle);
                },
                Err(e) => {
                    println!("error reading puzzle {:?}", e)
                }
            }
            line_idx += 1
        }
    }

    result

}
