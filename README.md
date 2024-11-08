# Slide Puzzle Challenge
Solve as many sliding puzzles out of 10,000 by using UP, LEFT, DOWN, and RIGHT movements only, and keeping the total number of moves within the limit specified in the downloaded slidepuzzle.txt file, you have 2 weeks to come up with an optimal solution.

## Puzzles 
Puzzles come in sizes N x M where N and M <= 7. The goal is to order the puzzles 1-9,a-z,A-Z with 0 (empty space) at the end. Some puzzles contain a '=' character which is an immovable wall.

## High Score: 36.06

# How to build and run this solver 

Prerequisites:

[Docker](https://docs.docker.com/engine/install/) and docker-compose 
[Rust](https://www.rust-lang.org/tools/install)

## Steps

1. Clone this repo and run the following command to start the puzzle cacher

```
docker-compose up -d 
```

2. Build the project using cargo
> It is HIGHLY recommended you use the --release flag or the solver will be very slow 

```
cargo build --release 
```


3. Run the solver 

```
./target/release/slidePuzzleSolver
```


# Current Strategy
The program utilizes several strategies to increase the solve speed

## Caching
Not all puzzles are guaranteed to be unique, and it is highly likely you will run this program multiple times. The goal is NOT to solve all puzzles in one program execution, it's to solve them within the given time frame (2 weeks). Thus, the puzzles you have already answered are cached to avoid unnecessary waste of time and processing power. 


## Multithreading 
This program uses the tokio runtime and rayon to parallelize puzzle solving. It is used in two ways:
- Running a solution algorithm on multiple puzzles at the same time (tokio)
- Running parallel exploration of puzzle nodes (rayon)

This utilizes a LOT of CPU power, which can be aleviated by adding a small few nanosecond delay after each loop iteration in the Puzzle.solve() method or limiting the number of worker threads. 


## Heuristics 
Currently, [manhattan distance]() is used to gauge proximity to a solution. There are a number of other heuristics including inversion which can be used, but I have found manhattan distance to work the best so far. 

You can alter the speed of the solver and the accuracy rate by altering the heuristic threshold. A lower threshold results in less answers but faster execution. A higher threshold increases the amount of nodes explored, leading to more solutions, but also raises execution time.


# Puzzle submission
This program utilizes the public MetroWeather API to download puzzles and grade answers.