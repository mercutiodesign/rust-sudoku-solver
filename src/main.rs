use log::{debug, error, info, log_enabled, trace, warn, Level};
use std::collections::HashMap;
use std::io::{self, BufRead};
use std::{fmt, mem};

// board parameters
const N: usize = 9;
const N_COLS: usize = N;
const N_ROWS: usize = N;
const N_NODE_COLS: usize = 4 * N_COLS * N_ROWS;
const N_NODE_COUNT: usize = N * N_NODE_COLS;
const N_GRID_COUNT: usize = (N + 1) * N_NODE_COLS + 1;

// index must be big enough to handle values up to 4 * N^3 (= N_NODE_COUNT)
type Index = u16;
type ColSize = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Board {
    cells: [[u8; N_COLS]; N_ROWS],
}

struct Node {
    left: Index,
    right: Index,
    up: Index,
    down: Index,
    column: Index,
}

struct View {
    nodes: Vec<Node>,
    sizes: [ColSize; N_NODE_COLS],
}

enum SearchState {
    Begin,
    SolutionFound,
    Cover,
    Uncover,
    Finished,
}

struct Table {
    view: View,
    given: Vec<Index>,    // row numbers * 4
    selected: Vec<Index>, // row numbers * 4
    search_state: SearchState,
}

impl Node {
    fn new(i: Index, column: Index) -> Self {
        Self {
            left: i,
            right: i,
            up: i,
            down: i,
            column,
        }
    }
}

fn row_num_to_coords(i: usize) -> (usize, usize, u8) {
    let x = i / (N * N);
    let y = (i / N) % N;
    let z = i % N;

    (x, y, z as u8 + 1)
}

#[allow(dead_code)]
fn row_num_to_name(i: usize) -> String {
    let (x, y, z) = row_num_to_coords(i);
    format!("R{}C{}#{}", x + 1, y + 1, z)
}

fn col_num_to_name(i: usize) -> String {
    let j = i % (N * N);
    let x = j / N;
    let y = j % N + 1;
    match i / (N * N) {
        0 => format!("R{}C{}", x + 1, y),
        1 => format!("R{}#{}", x + 1, y),
        2 => format!("C{}#{}", x + 1, y),
        3 => format!("B{}{}#{}", x / 3 + 1, x % 3 + 1, y),
        4 => format!("h{}", j),
        _ => panic!("{}?", i),
    }
}

impl From<&Table> for Board {
    fn from(table: &Table) -> Self {
        let mut cells = [[0; N]; N];
        for &i in table.given.iter().chain(table.selected.iter()) {
            let (x, y, z) = row_num_to_coords(i as usize / 4);
            cells[x][y] = z;
        }
        Self { cells }
    }
}

// long form representation of the board
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

// shorter representation of the board (just the values)
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

    Ok(board)
}

fn row_insert(nodes: &mut Vec<Node>, node: &mut Node, row: usize) {
    if let Some(n) = nodes.get_mut(row) {
        mem::swap(&mut n.left, &mut node.left);
    }
    if let Some(n) = nodes.get_mut(node.left as usize) {
        mem::swap(&mut n.right, &mut node.right);
    }
}

fn col_insert(nodes: &mut Vec<Node>, node: &mut Node, col: Index) {
    if let Some(n) = nodes.get_mut(col as usize) {
        mem::swap(&mut n.up, &mut node.up);
    }
    if let Some(n) = nodes.get_mut(node.up as usize) {
        mem::swap(&mut n.down, &mut node.down);
    }
}

fn check_col_count(nodes: &Vec<Node>, size: ColSize, col_start: Index) -> bool {
    let mut next = col_start;
    let mut count = 0;
    loop {
        debug_assert_eq!(nodes[next as usize].column, col_start);
        next = nodes[next as usize].down;
        count += 1;
        if next == col_start {
            break;
        }
    }
    debug_assert_eq!(count, size);
    count == size
}

fn add_node(nodes: &mut Vec<Node>, col_names: &mut HashMap<Index, Index>, row: usize, col: usize) {
    let new_i = nodes.len() as Index;
    let column = (col + N_NODE_COUNT) as Index;
    let col_start = *col_names.entry(column).or_insert(new_i);
    let mut node = Node::new(new_i, column);
    row_insert(nodes, &mut node, row);
    col_insert(nodes, &mut node, col_start);
    nodes.push(node);
    trace!("  add node {}", col_num_to_name(col));
}

fn col_numbers(i: usize, j: usize, z: usize) -> [usize; 4] {
    // row / col constraint
    // row-number constraint
    // col-number constraint
    // box-number constraint
    [
        i * N + j,
        (N + i) * N + z,
        (2 * N + j) * N + z,
        (3 * N + j / 3 * 3 + i / 3) * N + z,
    ]
}

impl From<&Board> for Table {
    fn from(board: &Board) -> Self {
        // cell index buildup:
        // for each row:
        // [row / col, row-number, col-number, box-number]
        let mut col_names = HashMap::new();
        let mut nodes = Vec::with_capacity(N_GRID_COUNT);

        for i in 0..N {
            for j in 0..N {
                for z in 0..N {
                    trace!("R{}C{}#{}", i + 1, j + 1, z + 1);
                    let row_num = (i * N + j) * N + z;
                    let row = row_num * 4;
                    debug_assert_eq!(row, nodes.len());

                    let columns = col_numbers(i, j, z);
                    for col in columns.into_iter() {
                        add_node(&mut nodes, &mut col_names, row, col);
                    }
                }
            }
        }

        debug_assert_eq!(nodes.len(), N_NODE_COUNT);

        // add column headers
        debug_assert_eq!(col_names.len(), N_NODE_COLS);
        trace!("adding column headers");
        for col in 0..N_NODE_COLS {
            add_node(&mut nodes, &mut col_names, N_NODE_COUNT, col);
        }

        trace!("adding root");
        add_node(&mut nodes, &mut col_names, N_NODE_COUNT, N_NODE_COLS);

        debug_assert_eq!(N_GRID_COUNT, nodes.len());

        let mut view = View {
            nodes,
            sizes: [N as ColSize; N_NODE_COLS],
        };

        let given = view.select_rows(board);
        Self {
            view,
            given,
            selected: vec![],
            search_state: SearchState::Begin,
        }
    }
}

impl Table {
    fn choose_column(&self, h_i: Index, mut j: Index) -> Index {
        debug_assert_ne!(h_i, j);
        let mut s = (N + 1) as ColSize;
        let mut c = j;
        while s > 0 && j != h_i {
            let j_size = self.view.sizes[j as usize - N_NODE_COUNT];
            debug_assert!(check_col_count(&self.view.nodes, j_size + 1, j));
            if j_size < s {
                c = j;
                s = j_size;
            }
            j = self.view.nodes[j as usize].right;
        }
        trace!(
            "chosen c: {:5} (s: {} = {:?})",
            self.view.col_name(c as usize),
            s,
            self.view.row_names(c)
        );
        c
    }

    fn search(&mut self) {
        // advance search state
        self.search_state = match self.search_state {
            SearchState::Begin => {
                trace!("search({})", self.selected.len());
                let h_i = N_GRID_COUNT - 1;
                let h_right = self.view.nodes[h_i].right;
                if h_i == (h_right as usize) {
                    SearchState::SolutionFound
                } else {
                    // Choose column object (lowest score)
                    let c = self.choose_column(h_i as Index, h_right);
                    let r = self.view.nodes[c as usize].down;
                    if r == c {
                        // invalid solution (0 entries in column)
                        SearchState::Uncover
                    } else {
                        self.view.cover_column(c);
                        self.selected.push(r);
                        SearchState::Cover
                    }
                }
            }
            SearchState::SolutionFound => SearchState::Uncover,
            SearchState::Cover => {
                let r = *self.selected.last().unwrap();
                trace!(" select {}", row_num_to_name((r / 4) as usize));
                let mut j = self.view.nodes[r as usize].right;
                while j != r {
                    let (next, column) = {
                        let n = &self.view.nodes[j as usize];
                        (n.right, n.column)
                    };
                    self.view.cover_column(column);
                    j = next;
                }
                SearchState::Begin
            }
            SearchState::Uncover => {
                if let Some(r) = self.selected.pop() {
                    let (mut j, r_down, c) = {
                        let n = &self.view.nodes[r as usize];
                        (n.left, n.down, n.column)
                    };
                    while j != r {
                        let (next, column) = {
                            let n = &self.view.nodes[j as usize];
                            (n.left, n.column)
                        };
                        self.view.uncover_column(column);
                        j = next;
                    }
                    if r_down == c {
                        self.view.uncover_column(c);
                        SearchState::Uncover
                    } else {
                        self.selected.push(r_down);
                        SearchState::Cover
                    }
                } else {
                    SearchState::Finished
                }
            }
            SearchState::Finished => SearchState::Finished,
        };
    }
}

impl View {
    fn select_rows(&mut self, board: &Board) -> Vec<Index> {
        let mut rows = vec![];
        for (i, row) in board.cells.iter().enumerate() {
            for (j, &cell) in row.iter().enumerate() {
                if cell < 1 {
                    continue;
                }

                trace!("cover row R{}C{}#{}", i + 1, j + 1, cell);
                rows.push(4 * (((i * N + j) * N) as Index + cell as Index - 1));

                let columns = col_numbers(i, j, (cell - 1) as usize);
                for col in columns.into_iter() {
                    if self.nodes[self.nodes[col + N_NODE_COUNT].left as usize].right as usize
                        != col + N_NODE_COUNT
                    {
                        warn!(
                            "invalid puzzle: {} is repeatedly covered",
                            self.col_name(col + N_NODE_COUNT)
                        );
                    } else {
                        self.cover_column((col + N_NODE_COUNT) as Index);
                    }
                }
            }
        }
        rows
    }

    fn col_name(&self, i: usize) -> String {
        debug_assert!(i >= N_NODE_COUNT);
        col_num_to_name(i - N_NODE_COUNT)
    }

    fn row_names(&self, c: Index) -> Vec<String> {
        let mut res = vec![];
        let mut next = self.nodes[c as usize].down;
        while next != c {
            res.push(row_num_to_name(next as usize / 4));
            next = self.nodes[next as usize].down;
        }
        res
    }

    fn cover_column(&mut self, column: Index) {
        trace!(
            "  cover col {} (s: {})",
            self.col_name(column as usize),
            self.sizes[column as usize - N_NODE_COUNT]
        );
        debug_assert!(check_col_count(
            &self.nodes,
            self.sizes[column as usize - N_NODE_COUNT] + 1,
            column
        ));
        let (c_right, c_left, mut i) = {
            let c = &self.nodes[column as usize];
            (c.right, c.left, c.down)
        };
        self.nodes[c_right as usize].left = c_left;
        self.nodes[c_left as usize].right = c_right;
        while i != column {
            let (mut j, i_down) = {
                let n = &self.nodes[i as usize];
                (n.right, n.down)
            };
            while j != i {
                let (j_right, j_up, j_down, j_c) = {
                    let n = &self.nodes[j as usize];
                    (n.right, n.up, n.down, n.column)
                };
                self.nodes[j_down as usize].up = j_up;
                self.nodes[j_up as usize].down = j_down;
                self.sizes[j_c as usize - N_NODE_COUNT] -= 1;
                j = j_right;
            }

            i = i_down;
        }
    }

    fn uncover_column(&mut self, column: Index) {
        trace!("  uncover col {}", self.col_name(column as usize));
        let (c_right, c_left, mut i) = {
            let c = &self.nodes[column as usize];
            (c.right, c.left, c.up)
        };
        while i != column {
            let (mut j, i_up) = {
                let n = &self.nodes[i as usize];
                (n.left, n.up)
            };
            while j != i {
                let (j_left, j_up, j_down, j_c) = {
                    let n = &self.nodes[j as usize];
                    (n.left, n.up, n.down, n.column)
                };
                self.nodes[j_down as usize].up = j;
                self.nodes[j_up as usize].down = j;
                self.sizes[j_c as usize - N_NODE_COUNT] += 1;
                j = j_left;
            }

            i = i_up;
        }
        self.nodes[c_right as usize].left = column;
        self.nodes[c_left as usize].right = column;
    }
}

impl Iterator for Table {
    type Item = Board;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.search_state {
                SearchState::SolutionFound => {
                    self.search();
                    return Some(Board::from(&*self));
                }
                SearchState::Finished => return None,
                _ => self.search(),
            }
        }
    }
}

fn main() {
    pretty_env_logger::init();
    let board = read_board().unwrap();
    debug!("solving:\n{:b}", board);

    let mut table = Table::from(&board);

    // ensure that we can reconstruct the board from the table:
    debug_assert_eq!(board, Board::from(&table));

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
