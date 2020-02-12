use log::{debug, info, trace, error};
use std::fmt;
use std::io::{self, BufRead};

#[derive(Debug, Clone, Copy)]
struct Board {
    cells: [[u8; 9]; 9],
    possibilities: [[[bool; 9]; 9]; 9],
}

struct Trace {
    pre_board: Board,
    possibility: PossiblePath,
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

impl fmt::Binary for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for row in &self.cells {
            if first {
                first = false;
            } else {
                writeln!(f)?
            }
            for cell in row {
                if *cell == 0 {
                    write!(f, ".")?
                } else {
                    write!(f, "{}", cell)?
                }
            }
        }
        Ok(())
    }
}

fn clear_value(possibilities: &mut [[[bool; 9]; 9]; 9], i: usize, j: usize, cell: &u8) {
    trace!("{} {}: clearing {}", i, j, cell);

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

#[derive(Debug, Clone, Copy)]
struct PossiblePath {
    i: usize,
    j: usize,
    v: u8,
}

enum SearchResult {
    Invalid,
    Changed,
    Finished,
    Possibility(Trace),
}

fn read_board() -> io::Result<Board> {
    let mut board = Board {
        cells: [[0; 9]; 9],
        possibilities: [[[true; 9]; 9]; 9],
    };

    let mut i = 0;
    let stdin = io::stdin();
    for maybe_line in stdin.lock().lines() {
        let mut j = 0;
        let line = maybe_line?;
        for ch in line.chars() {
            if ch == '!' || ch == '-' {
                continue;
            }
            match ch.to_digit(10) {
                Some(d) => {
                    if i >= board.cells.len() {
                        error!("line is out of bounds (i: {}) {}", i, line);
                    }
                    if j >= board.cells[i].len() {
                        error!("column is out of bounds (i: {}, j: {}, ch: {}) {}", i, j, ch, line);
                    }
                    board.cells[i][j] = d as u8;
                }
                None => {}
            }
            j += 1;
        }
        if j > 0 {
            i += 1;
        }
    }

    return Ok(board);
}

impl Board {
    fn find_possibilities(&mut self) -> bool {
        for (i, row) in self.cells.iter().enumerate() {
            for (j, cell) in row.iter().enumerate() {
                if *cell != 0 {
                    clear_value(&mut self.possibilities, i, j, cell);
                }
            }
        }

        let mut choices: Vec<Trace> = vec![];
        loop {
            match self.enter_fields() {
                SearchResult::Invalid => match choices.pop() {
                    Some(choice) => {
                        self.cells = choice.pre_board.cells;
                        self.possibilities = choice.pre_board.possibilities;
                        self.possibilities[(choice.possibility.v - 1) as usize]
                            [choice.possibility.i][choice.possibility.j] = false;
                        debug!("Backtracked   {:?} [c={}]", choice.possibility, choices.len());
                    }
                    None => {
                        info!("Could not solve board");
                        return false;
                    }
                },
                SearchResult::Changed => {
                    trace!("Changed, running again");
                }
                SearchResult::Finished => {
                    info!("Finished!");
                    return true;
                }
                SearchResult::Possibility(trace) => {
                    debug!("Branched into {:?} [c={}]", trace.possibility, choices.len());
                    choices.push(trace);
                }
            }
        }
    }

    fn enter_fields(&mut self) -> SearchResult {
        let mut cleared = false;
        let mut possible_path = PossiblePath { i: 0, j: 0, v: 0 };
        for (i, row) in self.cells.iter_mut().enumerate() {
            for (j, cell) in row.iter_mut().enumerate() {
                if *cell == 0 {
                    // check possible numbers
                    let x: Vec<bool> = self.possibilities.iter().map(|a| a[i][j]).collect();
                    let s = x.iter().filter(|&&x| x).count();
                    if s == 0 {
                        trace!("Invalid: no more choices for {} {}", i, j);
                        return SearchResult::Invalid;
                    }
                    if s <= 1 {
                        let pos = x.iter().position(|&v| v).unwrap() + 1;
                        trace!("{} {} only has one possibility: {}", i, j, pos);
                        *cell = pos as u8;
                        clear_value(&mut self.possibilities, i, j, cell);
                        cleared = true;
                    } else {
                        possible_path.i = i;
                        possible_path.j = j;
                        for (n, v) in x.iter().enumerate() {
                            // check if we're unique in this row / column / block
                            if *v {
                                let arr = self.possibilities[n];
                                let row_count = arr[i].iter().filter(|&&x| x).count();
                                let col_count = arr.iter().filter(|row| row[j]).count();
                                if row_count == 1 || col_count == 1 {
                                    let pos = n + 1;
                                    trace!("{} {} alone in row / col: {}", i, j, pos);
                                    *cell = pos as u8;
                                    clear_value(&mut self.possibilities, i, j, cell);
                                    cleared = true;
                                    break;
                                } else {
                                    possible_path.v = (n + 1) as u8;
                                }
                            }
                        }
                    }
                }
            }
        }

        if cleared {
            return SearchResult::Changed;
        } else if possible_path.v == 0 {
            return SearchResult::Finished;
        } else {
            let trace = Trace {
                pre_board: *self,
                possibility: possible_path,
            };

            let cell = &mut self.cells[possible_path.i][possible_path.j];
            *cell = possible_path.v;
            clear_value(
                &mut self.possibilities,
                possible_path.i,
                possible_path.j,
                cell,
            );
            return SearchResult::Possibility(trace);
        }
    }
}

fn main() {
    pretty_env_logger::init();
    let mut board = read_board().unwrap();
    debug!("solving:\n{:b}", board);

    board.find_possibilities();

    println!("{:b}", board);
}
