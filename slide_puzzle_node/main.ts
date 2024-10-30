import { ensureFile } from '@std/fs';
enum Direction {
  UP = "U",
  DOWN = "D",
  LEFT = "L",
  RIGHT = "R",
}


class PuzzleError implements Error {
    public name: string;
    public message: string;

    constructor(message: string) {
        this.name = "PuzzleError";
        this.message = message;
    }
}

enum Rune {
  EMPTY,
  WALL,
  VALUE
}

class Tile {

  public readonly rune: Rune;

  public static wall(): Tile {
    return new Tile('=')
  }

  public static space(): Tile {
    return new Tile('0')
  }

  constructor(public readonly raw: string) {
    if (raw.length !== 1) {
      throw new PuzzleError("Invalid tile");
    }

    switch (raw) {
      case "0":
        this.rune = Rune.EMPTY;
        break;
      case "=":
        this.rune = Rune.WALL;
        break;
      default:
        this.rune = Rune.VALUE;
        break;
    }
  }

  public rank(): number {

    const alphabet = "abcdefghijklmnopqrstuvwxyz";
    const alphabetUpper = alphabet.toUpperCase();
    const numerics = "0123456789";

    if (numerics.includes(this.raw)) {
      return Number(this.raw);
    }

    if (alphabet.includes(this.raw)) {
      return alphabet.indexOf(this.raw) + 10;
    }

    if (alphabetUpper.includes(this.raw)) {
      return alphabetUpper.indexOf(this.raw) + 36;
    }

    if (this.raw === "0") {
      return 62;
    }

    if (this.raw === "=") {
      return -1;
    }

    return -1;
  }

  public equals(other: Tile): boolean {
      return this.raw === other.raw;
  }

  public compare(other: Tile): number {
      return this.rank() - other.rank();
  }
}

interface Heuristic {
  get_heuristic(): number;
}

class Puzzle implements Heuristic{
  public readonly width?: number;
  public readonly height?: number;

  private _tiles: Tile[] = []
  constructor(raw?: string, width?: number, height?: number) {

    if (raw) {
      this.width = Number(raw.charAt(0));
      this.height = Number(raw.charAt(2));

      this._tiles = raw.substring(4).split("").map((tile) => new Tile(tile));
    }

    if (width && height) {
      this.height = height
      this.width = width
    }

  }

  public set tiles(tiles: Tile[]) {
    this._tiles = tiles
  }

  public get tiles(): Tile[] {
    return this._tiles;
  }

  private spaceIdx(): number {
    return this.tiles.indexOf(Tile.wall());
  }
  private legalMoves(): Direction[] {
    let legalMoves = [Direction.UP, Direction.DOWN, Direction.LEFT, Direction.RIGHT];

    const spaceIdx = this.spaceIdx();
    const row = spaceIdx / this.width!;
    const col = spaceIdx % this.width!;

    if (row == 0) {
      legalMoves = legalMoves.filter(d => d != Direction.UP);
    }

    if (row == this.height! - 1) {
      legalMoves = legalMoves.filter(d => d != Direction.DOWN);
    }

    if (col == 0) {
      legalMoves = legalMoves.filter(d => d != Direction.LEFT);
    }

    if (col == this.width! - 1) {
      legalMoves = legalMoves.filter(d => d != Direction.RIGHT);
    }

    if (spaceIdx == 0) {
      if (spaceIdx == 0) {
        legalMoves = legalMoves.filter(d => d != Direction.LEFT && d != Direction.UP);
      }
    }

    const rightTile = this.tiles[spaceIdx + 1]
    if (rightTile && rightTile.rune == Rune.WALL) {
      legalMoves = legalMoves.filter(d => d != Direction.RIGHT)
    }

    const leftTile = this.tiles[spaceIdx -1]
    if (leftTile && leftTile.rune == Rune.WALL) {
      legalMoves = legalMoves.filter(d => d != Direction.LEFT)
    }


    return legalMoves
  }

  get_heuristic(): number {
    return 0;
  }

  private manhattan_distance(tile: Tile): number {
    if (tile.rune == Rune.WALL) {
      return 0
    }

    let idx = this.tiles.indexOf(tile)


  }

  private serialized(): string {
    return this.tiles.map((t) => t.raw).join("")
  }

  solved(): Puzzle {

    const valueTiles = this.tiles.filter(t => t.rune != Rune.WALL && t.rune != Rune.EMPTY)

    const sorted = valueTiles.sort((a, b) => {
        if (a.rank() < b.rank()) {
          return -1
        }

        if (a.rank() > b.rank()) {
          return 1;
        }

        return 0;
    })

    sorted.push(Tile.space());

    for (let i = 0; i < this.tiles.length; i++) {
      if (this.tiles[i].rune == Rune.WALL) {
        sorted[i] = Tile.wall();
      }
    }

    const puzzle = new Puzzle(this._tiles)
    solved.tiles = sorted

    return solved

  }
}

async function solve() {
  console.log('solved')
}

await solve();