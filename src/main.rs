use std::fmt;

#[derive(Debug)]
struct Board {
    cells: [[u8; 9]; 9],
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter<>) -> fmt::Result {
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

fn main() {
    let board = Board {
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
    };
    println!("{}", board);
}
