#[derive(Debug, Clone, Copy, PartialEq, Hash)]
pub enum Rune {
    VALUE,
    SPACE,
    WALL
}

impl Rune {
    pub fn from_char(char: char) -> Self {
        match char {
            '=' => Rune::WALL,
            '0' => Rune::SPACE,
            _ => Rune::VALUE
        }
    }
}

#[derive(Debug, Clone, Copy, Hash, PartialEq)]
pub struct Tile {
    pub raw: char,
    pub rune: Rune,
    pub solved_idx: Option<usize>
}

pub fn serialize_tiles(tiles: &Vec<Tile>) -> String {
    tiles.iter().map(|t| t.raw).collect()
}


impl Tile {
    pub fn new(char: char) -> Self {

        Self {
            raw: char,
            rune: Rune::from_char(char),
            solved_idx: None
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