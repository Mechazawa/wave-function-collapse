#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

pub struct Grid<T> {
    data: Vec<T>,
    width: usize,
    height: usize,
}

pub struct GridIter<'a, T> {
    grid: &'a Grid<T>,
    pos: usize,
}

impl<T> Grid<T> {
    pub fn new<F: FnOnce(usize, usize) -> T>(width: usize, height: usize, initializer: F) -> Self {
        let mut data = Vec::with_capacity(width * height);

        for y in 0..height {
            for x in 0..width {
                data.push(initializer(x, y));
            }
        }

        Self {
            data,
            width,
            height,
        }
    }

    pub fn size(&self) -> usize {
      self.width * self.height
    }

    pub fn iter(&self) -> GridIter<T> {
        GridIter { grid: self, pos: 0 }
    }

    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        let index = x + (y * self.width);

        self.data.get(index)
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            Err("Cell out of range")?
        }

        let index = x + (y * self.width);

        self.data[index] = value;

        Ok(())
    }

    pub fn get_neighbors(&self, x: usize, y: usize) -> Vec<(Direction, &T)> {
        let output = Vec::with_capacity(4);

        for direction in [
            Direction::Up,
            Direction::Down,
            Direction::Left,
            Direction::Right,
        ] {
            if let Some(value) = self.get_neighbor(x, y, direction) {
                output.push((direction, value));
            }
        }

        output
    }

    pub fn get_neighbor(&self, x: usize, y: usize, direction: Direction) -> Option<&T> {
        let (x, y) = match direction {
            Direction::Up => {
                if y == 0 {
                    None
                } else {
                    Some((x, y - 1))
                }
            }
            Direction::Down => {
                if y + 1 >= self.height {
                    None
                } else {
                    Some((x, y + 1))
                }
            }
            Direction::Left => {
                if x == 0 {
                    None
                } else {
                    Some((x - 1, y))
                }
            }
            Direction::Right => {
                if x + 1 >= self.width {
                    None
                } else {
                    Some((x + 1, y))
                }
            }
        }?;

        self.get(x, y)
    }
}

impl<'a, T> IntoIterator for &'a Grid<T> {
    type Item = (usize, usize, &'a T);
    type IntoIter = GridIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> Iterator for GridIter<'a, T> {
    type Item = (usize, usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.grid.data.len() {
            None
        } else {
            let x = self.pos % self.grid.width;
            let y = self.pos / self.grid.width;
            let value = &self.grid.data[self.pos];

            self.pos += 1;

            Some((x, y, value))
        }
    }
}
