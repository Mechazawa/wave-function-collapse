
#[derive(Debug)]
struct EnumMap<E, T> 
where E: PartialEq {
    values: Vec<Option<(E, T)>>,
}

impl<E, T> EnumMap<E, T>
where E: PartialEq {
    pub fn new(variants: usize) -> Self {
        Self {
            // todo fixed size array
            values: Vec::with_capacity(variants),
        }   
    }

    pub fn insert(&mut self, key: E, value: T) {
        
    }

    fn find_index(&self, key: &E) -> Option<usize> {
        for i in 0..self.values.len() {
            // for i in 0..self.variants {
            let value = match &self.values[i] {
                None => continue,
                Some((e, _)) => e,
            };

            if value == key {
                return Some(i);
            }
        }

        None
    }
}