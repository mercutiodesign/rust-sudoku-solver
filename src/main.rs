#[macro_use]
extern crate log;
use bit_set::BitSet;
use log::{debug, error, info, trace, Level};
use std::fmt;
use std::io::{self, BufRead};
// use hibitset::BitSet;

const N: usize = 9;
const N_COLS: usize = N;
const N_ROWS: usize = N;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Board {
    cells: [[u8; N_COLS]; N_ROWS],
}

#[derive(Clone)]
struct View {
    columns: Vec<BitSet>,
    selected: BitSet,
}

struct Trace {
    pre_view: View,
    row: usize,
}

enum SearchResult {
    Invalid,
    Finished,
    Selected(usize),
    Possibility(Trace),
}

struct Table {
    view: View,
    traces: Vec<Trace>,
}

fn row_num_to_coords(i: usize) -> (usize, usize, u8) {
    let x = i / (N * N);
    let y = (i / N) % N;
    let z = i % N;

    return (x, y, z as u8 + 1);
}

fn row_num_to_name(i: usize) -> String {
    let (x, y, z) = row_num_to_coords(i);
    return format!("R{}C{}#{}", x + 1, y + 1, z);
}

#[allow(dead_code)]
fn col_num_to_name(i: usize) -> String {
    let j = i % (N * N);
    let x = j / N;
    let y = j % N + 1;
    return match i / (N * N) {
        0 => format!("R{}C{}", x + 1, y),
        1 => format!("R{}#{}", x + 1, y),
        2 => format!("C{}#{}", x + 1, y),
        3 => format!("B{}{}#{}", x / 3 + 1, x % 3 + 1, y),
        _ => panic!("{}?", i),
    };
}

impl From<&View> for Board {
    fn from(view: &View) -> Self {
        let mut cells = [[0; N]; N];
        for i in view.selected.iter() {
            let (x, y, z) = row_num_to_coords(i);
            cells[x][y] = z;
        }
        return Self { cells: cells };
    }
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

fn read_board() -> io::Result<Board> {
    let mut board = Board {
        cells: [[0; N_COLS]; N_ROWS],
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
                    if i >= N_ROWS {
                        error!("line is out of bounds (i: {}) {}", i, line);
                    }
                    if j >= N_COLS {
                        error!(
                            "column is out of bounds (i: {}, j: {}, ch: {}) {}",
                            i, j, ch, line
                        );
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

fn select_rows(columns: &mut Vec<BitSet>, board: &Board) -> BitSet {
    let rows = board
        .cells
        .iter()
        .enumerate()
        .flat_map(|(i, row)| {
            row.iter().enumerate().filter_map(move |(j, &cell)| {
                if cell != 0 {
                    Some((i * N + j) * N + cell as usize - 1)
                } else {
                    None
                }
            })
        })
        .rev()
        .collect();

    let mut excluded = BitSet::new();
    columns.retain(|col| {
        let contains = col.intersection(&rows).next().is_some();
        if contains {
            excluded.union_with(col);
        }
        !contains
    });
    for col in columns {
        col.difference_with(&excluded);
    }
    return rows;
}

impl View {
    fn select_row(&mut self, row: usize) {
        let added = self.selected.insert(row);
        assert!(added, "already selected row: {}", row_num_to_name(row));

        let mut excluded = BitSet::new();
        self.columns.retain(|col| {
            let contains = col.contains(row);
            if contains {
                excluded.union_with(col);
            }
            !contains
        });
        assert!(
            !excluded.is_empty(),
            "invalid puzzle: {}",
            row_num_to_name(row)
        );
        for col in self.columns.iter_mut() {
            col.difference_with(&excluded);
        }
    }

    fn log_col_counts(&self) {
        if !(log_enabled!(Level::Trace)) {
            return;
        }
        let mut sorted_columns = self.columns.clone();
        sorted_columns.sort_by_key(|col| col.len());
        for col in sorted_columns.iter() {
            let row_names: Vec<String> = col.iter().map(row_num_to_name).collect();
            trace!("col: -> {} ({})", row_names.join(", "), row_names.len());
        }
    }

    fn next_move(&self) -> SearchResult {
        if let Some(col) = self.columns.iter().min_by_key(|col| col.len()) {
            let mut iter = col.iter();
            if let Some(row) = iter.next() {
                if iter.next().is_none() {
                    // only choice for this row
                    return SearchResult::Selected(row);
                } else {
                    return SearchResult::Possibility(Trace {
                        pre_view: self.clone(),
                        row: row,
                    });
                }
            } else {
                return SearchResult::Invalid;
            }
        } else {
            return SearchResult::Finished;
        }
    }
}

impl From<&Board> for Table {
    fn from(board: &Board) -> Self {
        let mut columns = Vec::with_capacity(N * N * 4);

        // row / col constraint
        for i in 0..N {
            for j in 0..N {
                // row i, col j
                let col = (0..N).map(|v| (i * N + j) * N + v).rev().collect();
                columns.push(col);
            }
        }

        // row-number constraint
        for i in 0..N {
            for j in 0..N {
                // row i, number j
                let col = (0..N).map(|v| (i * N + v) * N + j).rev().collect();
                columns.push(col);
            }
        }

        // col-number constraint
        for i in 0..N {
            for j in 0..N {
                // col i, number j
                let col = (0..N).map(|v| (v * N + i) * N + j).rev().collect();
                columns.push(col);
            }
        }

        // box-number constraint
        for b_i in 0..3 {
            for b_j in 0..3 {
                for j in 0..N {
                    // block b_i/b_j, number j
                    let col = (0..3)
                        .flat_map(|x| {
                            (0..3).map(move |y| ((b_i * 3 + x) * N + b_j * 3 + y) * N + j)
                        })
                        .rev()
                        .collect();
                    columns.push(col);
                }
            }
        }

        let selected = select_rows(&mut columns, board);
        let view = View { columns, selected };
        let traces = vec![];
        return Self { view, traces };
    }
}

impl Table {
    fn backtrack(&mut self) -> bool {
        return match self.traces.pop() {
            Some(trace) => {
                self.view = trace.pre_view;
                for col in self.view.columns.iter_mut() {
                    col.remove(trace.row);
                }
                debug!(
                    "Backtracked   {} [c={}]",
                    row_num_to_name(trace.row),
                    self.traces.len()
                );
                true
            }
            None => false,
        };
    }
}

impl Iterator for Table {
    type Item = Board;

    fn next(&mut self) -> Option<Self::Item> {
        self.backtrack();

        loop {
            match self.view.next_move() {
                SearchResult::Invalid => {
                    if !self.backtrack() {
                        return None;
                    }
                }
                SearchResult::Finished => {
                    return Some((&self.view).into());
                }
                SearchResult::Selected(row) => {
                    self.view.select_row(row);
                }
                SearchResult::Possibility(trace) => {
                    debug!(
                        "Branched into {} [c={}]",
                        row_num_to_name(trace.row),
                        self.traces.len()
                    );
                    self.view.select_row(trace.row);
                    self.traces.push(trace);
                }
            }
        }
    }
}

fn main() {
    pretty_env_logger::init();
    let board = read_board().unwrap();
    debug!("solving:\n{:b}", board);

    let mut table = Table::from(&board);

    let n_board = Board::from(&table.view);
    assert_eq!(board, n_board);

    table.view.log_col_counts();

    if log_enabled!(Level::Info) {
        for (i, solution) in table.enumerate() {
            info!("solution {}:\n{:b}", i + 1, solution);
        }
    } else {
        if let Some(solution) = table.next() {
            println!("{:b}", solution);
        } else {
            println!("No solution found");
            std::process::exit(2);
        }
    }
}
