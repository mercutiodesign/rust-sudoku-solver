use std::fmt;

#[derive(Debug, Clone, Copy)]
struct Board {
    cells: [[u8; 9]; 9],
    possibilities: [[[bool; 9]; 9]; 9],
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let divider = true;
        for (i, row) in self.cells.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if *cell == 0 {
                    write!(f, " ")?
                } else {
                    write!(f, "{}", cell)?
                }

                if j + 1 == row.len() {
                    writeln!(f, "")?
                } else if divider && j % 3 == 2 {
                    write!(f, " | ")?
                } else {
                    write!(f, " ")?
                }
            }
            if divider && i < self.cells.len() - 1 && i % 3 == 2 {
                writeln!(f, "------|-------|------")?
            }
        }
        Ok(())
    }
}

fn clear_value(possibilities: &mut [[[bool; 9]; 9]; 9], i: usize, j: usize, cell: &u8) {
    println!("{} {}: clearing {}", i, j, cell);

    // set all other possible values at this position to false
    for arr in possibilities.iter_mut() {
        arr[i][j] = false;
    }

    let val: usize = (*cell - 1).into();
    for (k, row) in possibilities[val].iter_mut().enumerate() {
        if k == i {
            for c in row {
                *c = false;
            }
        } else if k / 3 == i / 3 {
            for (m, c) in row.iter_mut().enumerate() {
                if m / 3 == j / 3 {
                    *c = false;
                }
            }
        } else {
            row[j] = false;
        }
    }
}

impl Board {
    fn find_possibilities(&mut self) {
        for (i, row) in self.cells.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if *cell != 0 {
                    clear_value(&mut self.possibilities, i, j, cell);
                }
            }
        }

        let mut cleared = true;
        while cleared {
            cleared = false;
            for (i, row) in self.cells.iter_mut().enumerate() {
                for (j, cell) in row.iter_mut().enumerate() {
                    if *cell == 0 {
                        // check possible numbers
                        let x: Vec<bool> = self.possibilities.iter().map(|a| a[i][j]).collect();
                        let s = x.iter().filter(|&&x| x).count();
                        if s == 0 {
                            panic!("no more choices for {} {}", i, j)
                        }
                        if s <= 1 {
                            let pos = x.iter().position(|&v| v).unwrap() + 1;
                            println!("{} {} only has one possibility: {}", i, j, pos);
                            *cell = pos as u8;
                            clear_value(&mut self.possibilities, i, j, cell);
                            cleared = true;
                        } else {
                            for (n, v) in x.iter().enumerate() {
                                // check if we're unique in this row / column / block
                                if *v {
                                    let arr = self.possibilities[n];
                                    // todo: maybe check for single value in cell block
                                    if arr[i].iter().filter(|&&x| x).count() == 1
                                        || arr.iter().filter(|row| row[j]).count() == 1
                                    {
                                        let pos = n + 1;
                                        println!("{} {} alone in row / col: {}", i, j, pos);
                                        *cell = pos as u8;
                                        clear_value(&mut self.possibilities, i, j, cell);
                                        cleared = true;
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn main() {
    let mut board = Board {
        cells: [
            [3, 0, 9, 7, 0, 0, 0, 4, 0],
            [0, 0, 5, 0, 0, 2, 0, 0, 0],
            [0, 4, 0, 0, 0, 0, 0, 0, 0],
            [5, 0, 0, 3, 0, 0, 0, 7, 0],
            [6, 0, 4, 0, 2, 5, 0, 0, 0],
            [0, 0, 1, 0, 0, 0, 0, 0, 0],
            [0, 1, 7, 5, 0, 0, 2, 0, 0],
            [9, 0, 0, 0, 0, 0, 0, 0, 0],
            [4, 0, 0, 0, 0, 0, 6, 3, 7],
        ],
        possibilities: [[[true; 9]; 9]; 9],
    };
    println!("{}", board);

    board.find_possibilities();

    println!("{}", board);
}
