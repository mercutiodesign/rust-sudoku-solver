#[macro_use]
extern crate log;
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

// columns
const N_CONSTRAINTS: usize = 4 * N_COLS * N_ROWS;

// rows
const N_POSSIBILITIES: usize = 9 * N_COLS * N_ROWS;

struct BinaryTable {
    columns: [[bool; N_POSSIBILITIES]; N_CONSTRAINTS],
}

#[derive(Clone, Copy)]
struct View {
    columns: [bool; N_CONSTRAINTS],
    rows: [bool; N_POSSIBILITIES],
    selected: [bool; N_POSSIBILITIES],
}

struct Trace {
    pre_view: View,
    possibility: usize,
}

enum SearchResult {
    Invalid,
    Finished,
    Possibility(Trace),
}

struct Table {
    table: BinaryTable,
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
        for (i, v) in view.selected.iter().enumerate() {
            if *v {
                let (x, y, z) = row_num_to_coords(i);
                cells[x][y] = z;
            }
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

impl View {
    fn select_row(&mut self, table: &BinaryTable, row: usize) {
        assert!(
            !self.selected[row],
            "already selected row: {}",
            row_num_to_name(row)
        );
        assert!(self.rows[row], "invalid puzzle: {}", row_num_to_name(row));
        for (i, col) in table.columns.iter().enumerate() {
            if col[row] {
                assert!(
                    self.columns[i],
                    "invalid puzzle: {} violates {}",
                    row_num_to_name(row),
                    col_num_to_name(i)
                );
                self.columns[i] = false;
                for (j, v) in col.iter().enumerate() {
                    if *v {
                        self.rows[j] = false;
                    }
                }
            }
        }
        self.selected[row] = true;
    }

    fn from(board: &Board, table: &BinaryTable) -> Self {
        let mut view = Self {
            columns: [true; N_CONSTRAINTS],
            rows: [true; N_POSSIBILITIES],
            selected: [false; N_POSSIBILITIES],
        };

        for (i, row) in board.cells.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell != 0 {
                    view.select_row(table, (i * N + j) * N + cell as usize - 1);
                }
            }
        }

        return view;
    }

    fn col_count(&self, col: usize, table: &BinaryTable) -> u8 {
        return table.columns[col]
            .iter()
            .zip(self.rows.iter())
            .map(|(&a, &b)| (a && b) as u8)
            .sum();
    }

    fn col_row<'a>(
        &'a self,
        col: usize,
        table: &'a BinaryTable,
    ) -> impl Iterator<Item = usize> + 'a {
        return table.columns[col]
            .iter()
            .zip(self.rows.iter())
            .enumerate()
            .filter(|(_, (&a, &b))| a && b)
            .map(|(i, _)| i);
    }

    fn active_columns<'a>(&'a self) -> impl Iterator<Item = usize> + 'a {
        return self
            .columns
            .iter()
            .enumerate()
            .filter(|(_, &v)| v)
            .map(|(i, _)| i);
    }

    fn log_col_counts(&self, table: &BinaryTable) {
        if !(log_enabled!(Level::Trace)) {
            return;
        }
        let mut counts: Vec<(usize, u8)> = self
            .active_columns()
            .map(|i| (i, self.col_count(i, table)))
            .collect();
        counts.sort_by_key(|(_, count)| *count);
        for (i, count) in counts.iter() {
            let row_names: Vec<String> = self
                .col_row(*i, table)
                .map(|r| row_num_to_name(r))
                .collect();
            trace!(
                "{}: -> {} ({})",
                col_num_to_name(*i),
                row_names.join(", "),
                count
            );
            assert_eq!(*count as usize, row_names.len());
        }
    }

    fn next_move(&self, table: &BinaryTable) -> SearchResult {
        if let Some(col) = self
            .active_columns()
            .min_by_key(|col| self.col_count(*col, table))
        {
            if let Some(row) = self.col_row(col, table).next() {
                return SearchResult::Possibility(Trace {
                    pre_view: *self,
                    possibility: row,
                });
            } else {
                return SearchResult::Invalid;
            }
        } else {
            return SearchResult::Finished;
        }
    }
}

fn col_sum(col: &[bool; N_POSSIBILITIES]) -> usize {
    return col.iter().filter(|&&x| x).count();
}

impl From<&Board> for Table {
    fn from(board: &Board) -> Self {
        let mut table = BinaryTable {
            columns: [[false; N_POSSIBILITIES]; N_CONSTRAINTS],
        };

        let mut cols = table.columns.iter_mut();

        // row / col constraint
        for i in 0..N {
            for j in 0..N {
                trace!("{:4}: R{}C{}", i * N + j, i + 1, j + 1);
                let col = cols.next().unwrap();
                for v in 0..N {
                    let row = (i * N + j) * N + v;
                    col[row] = true;
                }
            }
        }

        // row-number constraint
        for i in 0..N {
            for j in 0..N {
                trace!("{:4}: R{}#{}", i * N + j + N * N, i + 1, j + 1);
                let col = cols.next().unwrap();
                for v in 0..N {
                    let row = (i * N + v) * N + j;
                    col[row] = true;
                }
            }
        }

        // col-number constraint
        for i in 0..N {
            for j in 0..N {
                trace!("{:4}: C{}#{}", i * N + j + 2 * N * N, i + 1, j + 1);
                let col = cols.next().unwrap();
                for v in 0..N {
                    let row = (v * N + i) * N + j;
                    col[row] = true;
                }
            }
        }

        // box-number constraint
        for b_i in 0..3 {
            for b_j in 0..3 {
                for j in 0..N {
                    trace!(
                        "{:4}: B{}{}#{}",
                        (b_i * 3 + b_j) * N + j + 3 * N * N,
                        b_i + 1,
                        b_j + 1,
                        j + 1
                    );
                    let col = cols.next().unwrap();
                    for x in 0..3 {
                        for y in 0..3 {
                            let row = ((b_i * 3 + x) * N + b_j * 3 + y) * N + j;
                            col[row] = true;
                        }
                    }
                }
            }
        }

        assert!(cols.next().is_none(), "Haven't filled all columns");

        for (i, col) in table.columns.iter().enumerate() {
            assert_eq!(col_sum(col), N, "col {}", i);
        }

        let view = View::from(board, &table);
        let traces = vec![];
        return Self {
            table,
            view,
            traces,
        };
    }
}

impl Table {
    fn backtrack(&mut self) -> bool {
        return match self.traces.pop() {
            Some(trace) => {
                self.view = trace.pre_view;
                self.view.rows[trace.possibility] = false;
                debug!(
                    "Backtracked   {} [c={}]",
                    row_num_to_name(trace.possibility),
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
            match self.view.next_move(&self.table) {
                SearchResult::Invalid => {
                    if !self.backtrack() {
                        return None;
                    }
                }
                SearchResult::Finished => {
                    return Some((&self.view).into());
                }
                SearchResult::Possibility(trace) => {
                    debug!(
                        "Branched into {} [c={}]",
                        row_num_to_name(trace.possibility),
                        self.traces.len()
                    );
                    self.view.select_row(&self.table, trace.possibility);
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

    table.view.log_col_counts(&table.table);

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
