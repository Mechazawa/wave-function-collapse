use core::str::FromStr;
use std::mem;
use enum_map::{enum_map, Enum, EnumMap};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Enum)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    #[must_use]
    pub fn invert(&self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

pub type Position = (usize, usize);

pub type Neighbors<T> = EnumMap<Direction, T>;

#[derive(Debug, Copy, Clone)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

impl FromStr for Size {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (raw_width, raw_height) = s.split_once('x').ok_or(format!("invalid format: {s}"))?;

        let width = raw_width
            .parse::<usize>()
            .map_err(|_| format!("invalid width: {raw_width}"))?;
        let height = raw_height
            .parse::<usize>()
            .map_err(|_| format!("invalid height: {raw_height}"))?;

        Ok(Size { width, height })
    }
}

impl Size {
    #[must_use]
    pub fn uniform(size: usize) -> Self {
        Self {
            width: size,
            height: size,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Grid<T>
where
    T: Clone,
{
    data: Vec<T>,
    width: usize,
    height: usize,
}

pub struct GridIter<'a, T>
where
    T: Clone,
{
    grid: &'a Grid<T>,
    pos: usize,
    x: usize,
    y: usize,
}

impl<T> Grid<T>
where
    T: Clone,
{
    pub fn new<F: FnMut(usize, usize) -> T>(
        width: usize,
        height: usize,
        initializer: &mut F,
    ) -> Self {
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

    #[must_use]
    pub fn size(&self) -> usize {
        self.width * self.height
    }

    #[must_use]
    pub fn iter(&self) -> GridIter<'_, T> {
        GridIter {
            grid: self,
            pos: 0,
            x: 0,
            y: 0,
        }
    }

    #[must_use]
    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        let index = x + (y * self.width);

        self.data.get(index)
    }

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        let index = x + (y * self.width);

        self.data.get_mut(index)
    }

    pub fn replace(&mut self, x: usize, y: usize, value: T) -> Option<T> {
        let index = x + (y * self.width);

        if index >= self.data.len() {
            None
        } else {
            Some(mem::replace(&mut self.data[index], value))
        }
    }

    /// # Errors
    /// Returns an error if the coordinates are out of bounds.
    pub fn set(&mut self, x: usize, y: usize, value: T) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            Err("Cell out of range")?;
        }

        let index = x + (y * self.width);

        self.data[index] = value;

        Ok(())
    }

    #[must_use]
    pub fn get_neighbors(&self, x: usize, y: usize) -> Neighbors<Option<&T>> {
        enum_map! {
            Direction::Up => self.get_neighbor(x, y, Direction::Up),
            Direction::Down => self.get_neighbor(x, y, Direction::Down),
            Direction::Left => self.get_neighbor(x, y, Direction::Left),
            Direction::Right => self.get_neighbor(x, y, Direction::Right),
        }
    }

    #[must_use]
    pub fn get_neighbor_positions(&self, x: usize, y: usize) -> Neighbors<Option<Position>> {
        enum_map! {
            Direction::Up => self.get_neighbor_position(x, y, Direction::Up),
            Direction::Down => self.get_neighbor_position(x, y, Direction::Down),
            Direction::Left => self.get_neighbor_position(x, y, Direction::Left),
            Direction::Right => self.get_neighbor_position(x, y, Direction::Right),
        }
    }

    #[must_use]
    pub fn get_neighbor_position(
        &self,
        x: usize,
        y: usize,
        direction: Direction,
    ) -> Option<Position> {
        match direction {
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
        }
    }

    #[must_use]
    pub fn get_neighbor(&self, x: usize, y: usize, direction: Direction) -> Option<&T> {
        let (lx, ly) = self.get_neighbor_position(x, y, direction)?;

        self.get(lx, ly)
    }

    #[must_use]
    pub fn width(&self) -> usize {
        self.width
    }

    #[must_use]
    pub fn height(&self) -> usize {
        self.height
    }

    /// # Panics
    /// Panics if accessing out of bounds coordinates.
    #[must_use]
    pub fn slice(&self, x: usize, y: usize, width: usize, height: usize) -> Grid<&T> {
        Grid::new(
            width.min(self.width() - x), 
            height.min(self.height() - y), 
            &mut |x, y| self.get(x, y).unwrap()
        )
    }

    #[must_use]
    pub fn chunked(&self, chunk_width: usize, chunk_height: usize) -> Vec<Grid<&T>> {
        let mut output = vec![];

        for x in (0..self.width()).step_by(chunk_width) {
            for y in (0..self.height()).step_by(chunk_height) {
                output.push(self.slice(x, y, chunk_width, chunk_height));
            }
        }

        output
    }
    
    /// Efficiently reset all grid cells to default value without reallocating
    pub fn reset_to_default(&mut self) 
    where
        T: Default,
    {
        for cell in &mut self.data {
            *cell = T::default();
        }
    }
}

impl<'a, T> IntoIterator for &'a Grid<T>
where
    T: Clone,
{
    type Item = (usize, usize, &'a T);
    type IntoIter = GridIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> Iterator for GridIter<'a, T>
where
    T: Clone,
{
    type Item = (usize, usize, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.grid.data.len() {
            None
        } else {
            let value = &self.grid.data[self.pos];
            let x = self.x;
            let y = self.y;

            self.x += 1;

            if self.x >= self.grid.width() {
                self.x = 0;
                self.y += 1;
            }

            self.pos += 1;

            Some((x, y, value))
        }
    }
}
