// Initialize an array to store moves
let moveHistory = [];

document.getElementById('start-button').addEventListener('click', function() {
    const input = document.getElementById('puzzle-input').value;
    if (input.length !== 9) {
        alert('Please enter exactly 9 characters.');
        return;
    }
    const characters = input.split('');
    moveHistory = []; // Reset move history when starting a new puzzle
    createPuzzle(characters);
});

function createPuzzle(chars) {
    const puzzleContainer = document.getElementById('puzzle');
    puzzleContainer.innerHTML = '';
    puzzleContainer.classList.remove('hidden');

    chars.forEach((char, index) => {
        const tile = document.createElement('div');
        tile.classList.add('tile');

        if (char === '0') {
            tile.classList.add('empty');
            tile.textContent = '';
        } else {
            tile.textContent = char;
            tile.addEventListener('click', function() {
                moveTile(index);
            });
        }
        puzzleContainer.appendChild(tile);
    });

    // Show the Save Moves button
    document.getElementById('save-button').classList.remove('hidden');
}

function moveTile(index) {
    const tiles = Array.from(document.getElementsByClassName('tile'));
    const emptyIndex = tiles.findIndex(tile => tile.classList.contains('empty'));

    const validMoves = getValidMoves(emptyIndex);

    if (validMoves.includes(index)) {
        recordMove(index, emptyIndex);
        swapTiles(tiles, index, emptyIndex);
    }
}

function swapTiles(tiles, index1, index2) {
    const tile1 = tiles[index1];
    const tile2 = tiles[index2];

    // Swap text content
    [tile1.textContent, tile2.textContent] = [tile2.textContent, tile1.textContent];

    // Swap classes
    tile1.classList.toggle('empty');
    tile2.classList.toggle('empty');

    // Update event listeners
    if (tile1.classList.contains('empty')) {
        tile1.removeEventListener('click', tile1.clickHandler);
    } else {
        tile1.clickHandler = function() {
            moveTile(index1);
        };
        tile1.addEventListener('click', tile1.clickHandler);
    }

    if (tile2.classList.contains('empty')) {
        tile2.removeEventListener('click', tile2.clickHandler);
    } else {
        tile2.clickHandler = function() {
            moveTile(index2);
        };
        tile2.addEventListener('click', tile2.clickHandler);
    }
}

function getValidMoves(emptyIndex) {
    const moves = [];
    const row = Math.floor(emptyIndex / 3);
    const col = emptyIndex % 3;

    if (row > 0) moves.push(emptyIndex - 3); // Up
    if (row < 2) moves.push(emptyIndex + 3); // Down
    if (col > 0) moves.push(emptyIndex - 1); // Left
    if (col < 2) moves.push(emptyIndex + 1); // Right

    return moves;
}

// New function to record moves
function recordMove(tileIndex, emptyIndex) {
    const direction = getMoveDirection(tileIndex, emptyIndex);
    moveHistory.push(direction);
}

// New function to determine move direction
function getMoveDirection(tileIndex, emptyIndex) {
    const difference = tileIndex - emptyIndex;
    switch (difference) {
        case -3:
            return 'D'; // Tile moved Down (empty moved Up)
        case 3:
            return 'U'; // Tile moved Up (empty moved Down)
        case -1:
            return 'R'; // Tile moved Right (empty moved Left)
        case 1:
            return 'L'; // Tile moved Left (empty moved Right)
        default:
            return '';
    }
}

// Add event listener to Save Moves button
document.getElementById('save-button').addEventListener('click', function() {
    saveMovesToFile();
});

// New function to save moves to a text file
function saveMovesToFile() {
    if (moveHistory.length === 0) {
        alert('No moves have been made yet.');
        return;
    }
    const movesString = moveHistory.join('');
    const blob = new Blob([movesString], { type: 'text/plain;charset=utf-8' });
    const filename = 'puzzle_moves.txt';

    // Create a link to download the file
    const link = document.createElement('a');
    link.href = URL.createObjectURL(blob);
    link.download = filename;

    // Trigger the download
    link.click();

    // Clean up
    URL.revokeObjectURL(link.href);
}