#[cfg(test)]
mod tests {
    use std::collections::{HashMap};
    use crate::{Direction, Heuristic, Puzzle, Tile};
    use crate::Direction::{DOWN, RIGHT, UP};
    use crate::util::read_puzzles;

    #[test]
    fn can_generate_legal_set_of_moves() {
        let puzzles = read_puzzles();

        let test_cases: HashMap<usize, Vec<Direction>> = [
            (1144, vec![Direction::UP, Direction::DOWN, Direction::LEFT, Direction::RIGHT]),
            (1145, vec![Direction::UP, Direction::DOWN, Direction::LEFT, Direction::RIGHT]),
            (1146, vec![Direction::LEFT, Direction::UP]),
            (1147, vec![Direction::UP, Direction::DOWN, Direction::RIGHT]),
            (1148, vec![Direction::DOWN, Direction::RIGHT]),
            (1149, vec![Direction::UP, Direction::DOWN, Direction::LEFT, Direction::RIGHT]),
            (1150, vec![Direction::LEFT, Direction::DOWN, Direction::RIGHT]),
            (1151, vec![Direction::UP, Direction::DOWN]),
            (1152, vec![Direction::LEFT, Direction::UP]),
            (1153, vec![Direction::UP, Direction::DOWN, Direction::LEFT, Direction::RIGHT]),
        ]
            .iter()
            .cloned()
            .collect();

        for (idx, expected_result) in test_cases {
            let puzzle = puzzles.get(idx).unwrap();
            let legal_moves = puzzle.legal_moves();
            assert_eq!(legal_moves.len(), expected_result.len(), "expected {:?} got {:?}", expected_result, legal_moves);
        }
    }

    #[test]
    fn calculates_manhattan_distance_correctly() {
        let puzzle_str = "3,3,123456708".to_string();

        let mut puzzle = Puzzle::from_str(puzzle_str);

        // Every tile is in place except 0 and 8 which are 1 move away
        for tile in &puzzle.tiles {
            let distance = puzzle.manhattan_distance(tile);

            if tile.raw == '0' || tile.raw == '8' {
                assert_eq!(distance, 1);
            } else {
                assert_eq!(distance, 0);
            }
        }

        // Sum up the distances to get the heuristic
        assert_eq!(puzzle.get_heuristic(), 2);

        puzzle.move_space(Direction::RIGHT).unwrap();

    }

    #[test]
    fn calculates_manhattan_distance_with_walls_correctly() {
        let puzzle_str = "3,3,12346075=".to_string();

        // initial puzzle:
        // 1 2 3
        // 4 6 0
        // 7 5 =

        let mut puzzle = Puzzle::from_str(puzzle_str);

        // 0 should be 2 moves away from its target position
        // 6 should be 1 move away from its target position
        // 5 should be 1 move away from its target position

        for tile in &puzzle.tiles {
            let distance = puzzle.manhattan_distance(tile);

            if tile.raw == '0' {
                assert_eq!(distance, 2);
            } else if tile.raw == '6' {
                assert_eq!(distance, 1);
            } else if tile.raw == '5' {
                assert_eq!(distance, 1);
            } else {
                assert_eq!(distance, 0);
            }
        }

        // Sum up the distances to get the heuristic
        assert_eq!(puzzle.get_heuristic(), 4);



    }


    #[test]
    fn move_space_swaps_correctly() {
        let mut puzzles = read_puzzles();
        let puzzle = puzzles.get_mut(1144).unwrap();
        let starting_idx = puzzle.space_idx();
        let target_idx = starting_idx + puzzle.width as usize;

        let space_idx = puzzle.space_idx();

        // Perform the move
        puzzle.move_space(Direction::DOWN).expect("illegal move made");

        assert_eq!(target_idx, puzzle.space_idx());

        // cannot move with wall
        assert!(puzzle.move_space(Direction::RIGHT).is_err());

        let puzzle_str = "4,3,12345678a0b=".to_string();

        let mut puzzle = Puzzle::from_str(puzzle_str);

        let start_idx = puzzle.space_idx();
        let expected_idx = start_idx - puzzle.width as usize;

        puzzle.move_space(UP).unwrap();

        assert_eq!(expected_idx, puzzle.space_idx());


    }

    #[test]
    fn check_is_solved_works() {

        let puzzle_str = "3,3,123456708".to_string();

        let mut puzzle = Puzzle::from_str(puzzle_str);

        puzzle.move_space(Direction::RIGHT).unwrap();


        assert!(puzzle.is_solved())
    }

    #[test]
    fn can_solve() {
        let puzzle_str = "4,3,12345678a0C=".to_string();

        let mut puzzle = Puzzle::from_str(puzzle_str);

        puzzle.solve();
        assert!(puzzle.is_solved());

    }
}