use core::str::FromStr;


#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Neighbors<T> {
    pub up: Option<T>,
    pub down: Option<T>,
    pub left: Option<T>,
    pub right: Option<T>,
}

impl<T> Neighbors<T> {
    pub fn get(&mut self, direction: Direction) -> Option<&mut T> {
        let value = match direction {
            Direction::Up => &mut self.up,
            Direction::Down => &mut self.down,
            Direction::Left => &mut self.left,
            Direction::Right => &mut self.right,
        };

        match value {
            None => None,
            Some(v) => Some(v),
        }
    }

    pub fn set(&mut self, direction: Direction, value: T) {
        match direction {
            Direction::Up => { self.up = Some(value); },
            Direction::Down => { self.down = Some(value); },
            Direction::Left => { self.left = Some(value); },
            Direction::Right => { self.right = Some(value); },
        }; 
    }

    pub fn delete(&mut self, direction: Direction) {
        match direction {
            Direction::Up => { self.up = None; },
            Direction::Down => { self.down = None; },
            Direction::Left => { self.left = None; },
            Direction::Right => { self.right = None; },
        }
    }

    pub fn is_empty(&self) -> bool {
        self.up.is_none() && 
        self.down.is_none() && 
        self.left.is_none() && 
        self.right.is_none()
    }

    pub fn count(&self) -> usize {
        let mut output = 0;
        
        if self.up.is_some() { output += 1; }
        if self.down.is_some() { output += 1; }
        if self.left.is_some() { output += 1; }
        if self.right.is_some() { output += 1; }
        
        output
    }

    pub fn list(&self) -> Vec<Direction> {
        let mut output = vec![];
        
        if self.up.is_some() { output.push(Direction::Up); }
        if self.down.is_some() { output.push(Direction::Down); }
        if self.left.is_some() { output.push(Direction::Left); }
        if self.right.is_some() { output.push(Direction::Right); }
        
        output
    }
}

impl<T: std::default::Default> Neighbors<T> {
    pub fn get_or_default(&mut self, direction: Direction) -> Option<&mut T> {
        let output = self.get(direction);

        match output {
            Some(value) => Some(value),
            None => {
                self.set(direction, Default::default());
                self.get(direction)
            }
        }
    }
}
    

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

    pub fn get_neighbors(&self, x: usize, y: usize) -> Vec<(Direction, &T)> {
        let mut output = Vec::with_capacity(4);

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
