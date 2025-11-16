const INIT_CAPACITY: usize = 8;

#[derive(Debug)]
pub struct VecPool<T> {
    pool: Vec<Vec<T>>,
}

impl<T> Default for VecPool<T> {
    fn default() -> Self {
        Self::new_fat()
    }
}

impl<T> VecPool<T> {
    pub fn new_fat() -> Self {
        Self {
            pool: (0..INIT_CAPACITY)
                .map(|_| Vec::with_capacity(INIT_CAPACITY))
                .collect(),
        }
    }

    pub fn get(&mut self) -> Vec<T> {
        self.pool
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(INIT_CAPACITY))
    }

    pub fn ret(&mut self, mut vec: Vec<T>) {
        vec.clear();
        self.pool.push(vec);
    }
}
