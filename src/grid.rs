use core::str::FromStr;
use enum_map::{Enum, EnumMap, enum_map};

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy, Enum)]
pub enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl Direction {
    pub fn invert(&self) -> Self {
        match self {
            Direction::Up => Direction::Down,
            Direction::Down => Direction::Up,
            Direction::Left => Direction::Right,
            Direction::Right => Direction::Left,
        }
    }
}

pub type Neighbors<T> = EnumMap<Direction, T>;

#[derive(Debug, Copy, Clone)]
pub struct Size {
    pub width: usize,
    pub height: usize,
}

impl FromStr for Size {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (raw_width, raw_height) = s.split_once('x').ok_or(format!("invalid format: {}", s))?;

        let width = raw_width
            .parse::<usize>()
            .map_err(|_| format!("invalid width: {}", raw_width))?;
        let height = raw_height
            .parse::<usize>()
            .map_err(|_| format!("invalid height: {}", raw_height))?;

        Ok(Size { width, height })
    }
}

#[derive(Debug, Clone)]
pub struct Grid<T>
where T: Clone {
    data: Vec<T>,
    width: usize,
    height: usize,
}

pub struct GridIter<'a, T>
where T: Clone  {
    grid: &'a Grid<T>,
    pos: usize,
}

impl<T> Grid<T> 
where T: Clone {
    pub fn new<F: FnMut(usize, usize) -> T>(width: usize, height: usize, initializer: &mut F) -> Self {
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

    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        let index = x + (y * self.width);

        self.data.get_mut(index)
    }

    pub fn set(&mut self, x: usize, y: usize, value: T) -> Result<(), &'static str> {
        if x >= self.width || y >= self.height {
            Err("Cell out of range")?
        }

        let index = x + (y * self.width);

        self.data[index] = value;

        Ok(())
    }

    pub fn get_neighbors(&self, x: usize, y: usize) -> Neighbors<Option<&T>> {
        enum_map! {
            Direction::Up => self.get_neighbor(x, y, Direction::Up),
            Direction::Down => self.get_neighbor(x, y, Direction::Down),
            Direction::Left => self.get_neighbor(x, y, Direction::Left),
            Direction::Right => self.get_neighbor(x, y, Direction::Right),
        }
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

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }
}

impl<'a, T> IntoIterator for &'a Grid<T>
where T: Clone  {
    type Item = (usize, usize, &'a T);
    type IntoIter = GridIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T> Iterator for GridIter<'a, T> 
where T: Clone {
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
