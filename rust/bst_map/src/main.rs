use std::{cmp::Ordering, collections::BTreeMap, time::Instant};

use rand::{Rng, SeedableRng};

struct BstMap<K, V> {
    root: Option<Box<Node<K, V>>>,
}

struct Node<K, V> {
    left: Option<Box<Self>>,
    right: Option<Box<Self>>,
    key: K,
    value: V,
}

impl<K, V> Node<K, V>
where
    K: Ord,
{
    fn new_leaf(k: K, v: V) -> Self {
        Self {
            left: None,
            right: None,
            key: k,
            value: v,
        }
    }

    fn insert(&mut self, k: K, v: V) {
        match k.cmp(&self.key) {
            Ordering::Equal => self.value = v,
            Ordering::Less => {
                if let Some(l) = &mut self.left {
                    l.insert(k, v);
                } else {
                    self.left = Some(Node::new_leaf(k, v).into());
                }
            }
            Ordering::Greater => {
                if let Some(r) = &mut self.right {
                    r.insert(k, v);
                } else {
                    self.right = Some(Node::new_leaf(k, v).into());
                }
            }
        }
    }

    fn get(&self, k: &K) -> Option<&V> {
        match k.cmp(&self.key) {
            Ordering::Equal => Some(&self.value),
            Ordering::Less => self.left.as_ref().and_then(|it| it.get(k)),
            Ordering::Greater => {
                self.right.as_ref().and_then(|it| it.get(k))
            }
        }
    }

    fn remove(
        this_ref: &mut Option<Box<Self>>,
        k: &K,
    ) -> Option<Box<Self>> {
        let this = this_ref.as_mut()?;

        match k.cmp(&this.key) {
            Ordering::Equal => Self::remove_self(this_ref),
            Ordering::Less => Node::remove(&mut this.left, k),
            Ordering::Greater => Node::remove(&mut this.right, k),
        }
    }

    fn remove_self(
        this_ref: &mut Option<Box<Self>>,
    ) -> Option<Box<Self>> {
        let this = this_ref.as_mut()?;

        match (&mut this.left, &mut this.right) {
            (None, None) => this_ref.take(),
            (None, Some(_)) => {
                let r = this.right.take().unwrap();
                this_ref.replace(r)
            }
            (Some(_), None) => {
                let l = this.left.take().unwrap();
                this_ref.replace(l)
            }
            (Some(l), Some(r)) => {
                if l.right.is_none() {
                    let mut l = this.left.take().unwrap();
                    let r = this.right.take().unwrap();
                    l.right = Some(r);
                    this_ref.replace(l)
                } else if r.left.is_none() {
                    let l = this.left.take().unwrap();
                    let mut r = this.right.take().unwrap();
                    r.left = Some(l);
                    this_ref.replace(r)
                } else {
                    let mut new_self =
                        Self::remove_left_most(&mut r.left).unwrap();
                    let l = this.left.take().unwrap();
                    let r = this.right.take().unwrap();
                    new_self.left = Some(l);
                    new_self.right = Some(r);
                    this_ref.replace(new_self)
                }
            }
        }
    }

    fn remove_left_most(
        this_ref: &mut Option<Box<Self>>,
    ) -> Option<Box<Self>> {
        let this = this_ref.as_mut()?;

        if this.left.is_some() {
            Self::remove_left_most(&mut this.left)
        } else {
            Self::remove_self(this_ref)
        }
    }

    fn entries<'a>(&'a self, out: &mut Vec<(&'a K, &'a V)>) {
        if let Some(left) = &self.left {
            left.entries(out);
        }

        out.push((&self.key, &self.value));

        if let Some(right) = &self.right {
            right.entries(out);
        }
    }
}

impl<K, V> BstMap<K, V>
where
    K: Ord,
{
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn insert(&mut self, k: K, v: V) {
        if let Some(root) = &mut self.root {
            root.insert(k, v);
        } else {
            self.root = Some(Node::new_leaf(k, v).into());
        }
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.root.as_ref().and_then(|it| it.get(k))
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        Node::remove(&mut self.root, k).map(|it| it.value)
    }

    pub fn entries(&self) -> Vec<(&K, &V)> {
        let mut out = vec![];
        if let Some(root) = &self.root {
            root.entries(&mut out);
        }
        out
    }
}

fn main() {
    let mut b = BTreeMap::<u8, i32>::new();

    let mut t = BstMap::<u8, i32>::new();

    let mut rng = rand::rngs::SmallRng::seed_from_u64(114);

    let mut last = Instant::now();
    for i in 0..=100_000_000 {
        if i % 100 == 0 && last.elapsed().as_secs_f32() > 1.0 {
            println!("{i}, {}", b.len());
            last = Instant::now();
        }

        if rng.random_bool(0.5) {
            let (k, v): (u8, i32) = rng.random();
            b.insert(k, v);
            t.insert(k, v);
        } else {
            let k: u8 = rng.random();
            assert_eq!(b.remove(&k), t.remove(&k));
        }

        for ele in b.iter() {
            if t.get(ele.0) != Some(ele.1) {
                dbg!(t.entries());
            }
            assert_eq!(t.get(ele.0), Some(ele.1));
        }
    }
}
