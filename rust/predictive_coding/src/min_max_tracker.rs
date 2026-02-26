use super::Fp;

pub struct MinMaxTracker {
    epsilon: Fp,
    min: Fp,
    max: Fp,
    count: usize,
}

impl MinMaxTracker {
    pub fn new(epsilon: Fp) -> Self {
        Self {
            epsilon,
            min: Fp::MAX,
            max: Fp::MIN,
            count: 0,
        }
    }

    pub fn update(&mut self, x: Fp) {
        let epsilon = x.abs() * self.epsilon;

        self.count += 1;
        if x < self.min - epsilon {
            self.min = x;
            self.count = 0;
        }
        if x > self.max + epsilon {
            self.max = x;
            self.count = 0;
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.epsilon)
    }

    pub fn range(&self) -> Fp {
        self.max - self.min
    }

    pub fn epsilon(&self) -> Fp {
        self.epsilon
    }

    pub fn min(&self) -> Fp {
        self.min
    }

    pub fn max(&self) -> Fp {
        self.max
    }

    pub fn count(&self) -> usize {
        self.count
    }
}
