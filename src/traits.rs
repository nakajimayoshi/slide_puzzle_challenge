pub(crate) mod puzzle {
    use colored::Colorize;
    use crate::puzzle::Puzzle;
    use crate::tile::Rune;

    pub trait DebugPrintable {
        fn debug_print(&self, manhattan_distance: bool);
    }

    impl DebugPrintable for Puzzle {
        fn debug_print(&self, manhattan_distance: bool) {
            let solved = self.solved();
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
                        if manhattan_distance {
                            match tile.rune {
                                Rune::WALL => print!(" {} ", "█"),
                                Rune::SPACE => print!(" {} ", self.manhattan_distance(tile, &solved)),
                                _ => print!(" {} ", self.manhattan_distance(tile, &solved)),
                            }
                        } else {
                            match tile.rune {
                                Rune::WALL => print!(" {} ", "█"),
                                Rune::SPACE => print!(" {} ", " ".green()),
                                _ => print!(" {} ", tile.raw),
                            }
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
    }


    pub trait Heuristic {
        fn get_heuristic(&self, solved_puzzle: &Puzzle) -> f32;
    }

    impl Heuristic for Puzzle {
        fn get_heuristic(&self, solved_puzzle: &Puzzle) -> f32 {
            let mut heuristic: f32 = 0.;

            for (idx, tile) in self.tiles.iter().enumerate() {
                heuristic+=self.manhattan_distance(tile, solved_puzzle) as f32;
                // heuristic+=2 * self.linear_conflicts(idx)
            }

            heuristic
        }
    }
}

pub(crate) mod tile {
    use std::cmp::Ordering;
    use crate::tile::Tile;

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
}