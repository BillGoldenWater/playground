use std::{
    cmp::Ordering, collections::BTreeMap, fmt::Debug,
    sync::atomic::AtomicUsize, time::Instant,
};

use rand::{Rng, SeedableRng};

struct AvlTreeMap<K, V> {
    root: Option<Box<Node<K, V>>>,
}

#[derive(Debug)]
struct Node<K, V> {
    left: Option<Box<Self>>,
    right: Option<Box<Self>>,
    balance_factor: i8,
    key: K,
    value: V,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BalanceResult {
    None,
    Balanced,
    BalancedDec,
}

impl BalanceResult {
    fn is_dec(&self) -> bool {
        matches!(self, Self::BalancedDec)
    }
}

impl<K, V> Node<K, V>
where
    K: Ord,
{
    fn new_leaf(k: K, v: V) -> Self {
        Self {
            left: None,
            right: None,
            balance_factor: 0,
            key: k,
            value: v,
        }
    }

    /// # Returns
    /// is height increased
    fn insert(&mut self, k: K, v: V) -> bool {
        match k.cmp(&self.key) {
            Ordering::Equal => {
                self.value = v;
                false
            }
            Ordering::Less => {
                let inc = if let Some(l) = &mut self.left {
                    let inc = l.insert(k, v);
                    let res = Self::handle_balancing(&mut self.left);
                    if res.is_dec() { false } else { inc }
                } else {
                    self.left = Some(Node::new_leaf(k, v).into());
                    true
                };

                self.balance_factor -= inc as i8;

                if self.balance_factor >= 0 { false } else { inc }
            }
            Ordering::Greater => {
                let inc = if let Some(r) = &mut self.right {
                    let inc = r.insert(k, v);
                    let res = Self::handle_balancing(&mut self.right);
                    if res.is_dec() { false } else { inc }
                } else {
                    self.right = Some(Node::new_leaf(k, v).into());
                    true
                };

                self.balance_factor += inc as i8;

                if self.balance_factor <= 0 { false } else { inc }
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
    ) -> Option<(Box<Self>, bool)> {
        let this = this_ref.as_mut()?;

        match k.cmp(&this.key) {
            Ordering::Equal => Self::remove_self(this_ref),
            Ordering::Less => match Node::remove(&mut this.left, k) {
                Some(mut it @ (_, dec)) => {
                    if dec {
                        this.balance_factor += 1;
                        if !this.is_bf_zero() {
                            let res = Self::handle_balancing(this_ref);
                            if !res.is_dec() {
                                it.1 = false;
                            }
                        }
                    }
                    Some(it)
                }
                None => None,
            },
            Ordering::Greater => match Node::remove(&mut this.right, k) {
                Some(mut it @ (_, dec)) => {
                    if dec {
                        this.balance_factor -= 1;
                        if !this.is_bf_zero() {
                            let res = Self::handle_balancing(this_ref);
                            if !res.is_dec() {
                                it.1 = false;
                            }
                        }
                    }
                    Some(it)
                }
                None => None,
            },
        }
    }

    fn remove_self(
        this_ref: &mut Option<Box<Self>>,
    ) -> Option<(Box<Self>, bool)> {
        let this = this_ref.as_mut()?;

        let wrap_dec = |it| (it, true);
        let wrap_nodec = |it| (it, false);
        let do_replace_ret =
            |this_ref: &mut Option<Box<Node<K, V>>>,
             new: Box<Node<K, V>>,
             bf_changed: bool| {
                let bf_zero = new.is_bf_zero();
                let ret = this_ref.replace(new);
                let res = Self::handle_balancing(this_ref);

                let wrap = if (bf_changed && bf_zero) || res.is_dec() {
                    wrap_dec
                } else {
                    wrap_nodec
                };
                ret.map(wrap)
            };

        match (&mut this.left, &mut this.right) {
            (None, None) => this_ref.take().map(wrap_dec),
            (None, Some(_)) => {
                let r = this.right.take().unwrap();
                this_ref.replace(r).map(wrap_dec)
            }
            (Some(_), None) => {
                let l = this.left.take().unwrap();
                this_ref.replace(l).map(wrap_dec)
            }
            (Some(l), Some(r)) => {
                if l.right.is_none() {
                    let mut l = this.left.take().unwrap();
                    let r = this.right.take().unwrap();
                    l.right = Some(r);

                    if l.left.is_some() {
                        // l's height == 2
                        let rh = this.balance_factor + 2;
                        l.balance_factor += rh;
                    } else {
                        // l's height == 1
                        let rh = this.balance_factor + 1;
                        l.balance_factor += rh;
                    }

                    do_replace_ret(this_ref, l, true)
                } else if r.left.is_none() {
                    let l = this.left.take().unwrap();
                    let mut r = this.right.take().unwrap();
                    r.left = Some(l);

                    if r.right.is_some() {
                        // r's height == 2
                        let lh = 2 - this.balance_factor;
                        r.balance_factor -= lh;
                    } else {
                        // r's height == 1
                        let lh = 1 - this.balance_factor;
                        r.balance_factor -= lh;
                    }

                    do_replace_ret(this_ref, r, true)
                } else {
                    let (mut new_this, mut dec) =
                        Self::remove_left_most(&mut r.left).unwrap();
                    if dec {
                        r.balance_factor += 1;
                        if !r.is_bf_zero() {
                            let res =
                                Self::handle_balancing(&mut this.right);
                            if !res.is_dec() {
                                dec = false;
                            }
                        }
                    }

                    let l = this.left.take().unwrap();
                    let r = this.right.take().unwrap();
                    new_this.left = Some(l);
                    new_this.right = Some(r);

                    let old_bf = this.balance_factor;
                    new_this.balance_factor = old_bf - dec as i8;

                    do_replace_ret(this_ref, new_this, dec)
                }
            }
        }
    }

    fn remove_left_most(
        this_ref: &mut Option<Box<Self>>,
    ) -> Option<(Box<Self>, bool)> {
        let this = this_ref.as_mut()?;

        if this.left.is_some() {
            let (node, mut dec) =
                Self::remove_left_most(&mut this.left).unwrap();
            if dec {
                this.balance_factor += 1;
                if !this.is_bf_zero() {
                    let res = Self::handle_balancing(this_ref);
                    if !res.is_dec() {
                        dec = false;
                    }
                }
            }
            Some((node, dec))
        } else {
            Self::remove_self(this_ref)
        }
    }

    fn is_bf_zero(&self) -> bool {
        self.balance_factor == 0
    }

    fn handle_balancing(
        this_ref: &mut Option<Box<Self>>,
    ) -> BalanceResult {
        // empty node can't be unbalanced
        let Some(this) = this_ref.as_mut() else {
            return BalanceResult::None;
        };

        if this.balance_factor.abs() <= 1 {
            return BalanceResult::None;
        }

        let dec = if this.balance_factor.signum() == 1 {
            let r = this.right.as_mut().unwrap();
            if r.balance_factor >= 0 {
                // RR
                Self::rotate_left(this_ref, true)
            } else {
                // RL
                Self::rotate_right_left(this_ref)
            }
        } else {
            let l = this.left.as_mut().unwrap();
            if l.balance_factor <= 0 {
                // LL
                Self::rotate_right(this_ref, true)
            } else {
                // LR
                Self::rotate_left_right(this_ref)
            }
        };

        if dec {
            BalanceResult::BalancedDec
        } else {
            BalanceResult::Balanced
        }
    }

    /// # Returns
    /// is height decreased
    fn rotate_left(
        this_ref: &mut Option<Box<Self>>,
        update_bf: bool,
    ) -> bool {
        let Some(this) = this_ref.as_mut() else {
            return false;
        };

        let mut r = this.right.take().unwrap();
        let rl = r.left.take();
        this.right = rl;
        let new_l = this_ref.take();
        let new_this = this_ref.insert(r);
        new_this.left = new_l;

        if !update_bf {
            return !new_this.is_bf_zero();
        }

        let bf_zero = new_this.is_bf_zero();
        let l = new_this.left.as_mut().unwrap();
        if bf_zero {
            l.balance_factor = 1;
            new_this.balance_factor = -1;
            false
        } else {
            l.balance_factor = 0;
            new_this.balance_factor = 0;
            true
        }
    }

    /// # Returns
    /// is height decreased
    fn rotate_right(
        this_ref: &mut Option<Box<Self>>,
        update_bf: bool,
    ) -> bool {
        let Some(this) = this_ref.as_mut() else {
            return false;
        };

        let mut l = this.left.take().unwrap();
        let lr = l.right.take();
        this.left = lr;
        let new_r = this_ref.take();
        let new_this = this_ref.insert(l);
        new_this.right = new_r;

        if !update_bf {
            return !new_this.is_bf_zero();
        }

        let bf_zero = new_this.is_bf_zero();
        let r = new_this.right.as_mut().unwrap();
        if bf_zero {
            r.balance_factor = -1;
            new_this.balance_factor = 1;
            false
        } else {
            r.balance_factor = 0;
            new_this.balance_factor = 0;
            true
        }
    }

    /// # Returns
    /// is height decreased
    fn rotate_right_left(this_ref: &mut Option<Box<Self>>) -> bool {
        let Some(this) = this_ref.as_mut() else {
            return false;
        };

        Self::rotate_right(&mut this.right, false);
        Self::rotate_left(this_ref, false);

        let this = this_ref.as_mut().unwrap();

        let bf_zero = this.is_bf_zero();
        let l = this.left.as_mut().unwrap();
        let r = this.right.as_mut().unwrap();
        if bf_zero {
            l.balance_factor = 0;
            r.balance_factor = 0;
        } else if this.balance_factor > 0 {
            l.balance_factor = -1;
            r.balance_factor = 0;
        } else {
            l.balance_factor = 0;
            r.balance_factor = 1;
        }
        this.balance_factor = 0;

        true
    }

    /// # Returns
    /// is height decreased
    fn rotate_left_right(this_ref: &mut Option<Box<Self>>) -> bool {
        let Some(this) = this_ref.as_mut() else {
            return false;
        };

        Self::rotate_left(&mut this.left, false);
        Self::rotate_right(this_ref, false);

        let this = this_ref.as_mut().unwrap();

        let bf_zero = this.is_bf_zero();
        let l = this.left.as_mut().unwrap();
        let r = this.right.as_mut().unwrap();
        if bf_zero {
            l.balance_factor = 0;
            r.balance_factor = 0;
        } else if this.balance_factor <= 0 {
            l.balance_factor = 0;
            r.balance_factor = 1;
        } else {
            l.balance_factor = -1;
            r.balance_factor = 0;
        }
        this.balance_factor = 0;

        true
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

    fn to_dot(&self, out: &mut String)
    where
        K: Debug,
    {
        let key = &self.key;
        out.push_str(&format!(
            r#"    "{key:?}" [label="{key:?} ({bf})"];
"#,
            bf = self.balance_factor
        ));

        if let Some(left) = &self.left {
            out.push_str(&format!(
                r#"    "{key:?}" -> "{:?}" [label="l"]
"#,
                left.key
            ));
            left.to_dot(out);
        }

        if let Some(right) = &self.right {
            out.push_str(&format!(
                r#"    "{key:?}" -> "{:?}" [label="r"]
"#,
                right.key
            ));
            right.to_dot(out);
        }
    }
}

impl<K, V> AvlTreeMap<K, V>
where
    K: Ord,
{
    pub fn new() -> Self {
        Self { root: None }
    }

    pub fn insert(&mut self, k: K, v: V) {
        if let Some(root) = &mut self.root {
            root.insert(k, v);
            Node::handle_balancing(&mut self.root);
        } else {
            self.root = Some(Node::new_leaf(k, v).into());
        }
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        self.root.as_ref().and_then(|it| it.get(k))
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let this = Node::remove(&mut self.root, k);
        this.map(|(it, _)| it.value)
    }

    pub fn entries(&self) -> Vec<(&K, &V)> {
        let mut out = vec![];
        if let Some(root) = &self.root {
            root.entries(&mut out);
        }
        out
    }

    pub fn to_dot(&self) -> String
    where
        K: Debug,
    {
        let mut out = String::new();
        out.push_str(
            r"digraph Tree {
    node [shape=record];

",
        );

        if let Some(root) = &self.root {
            root.to_dot(&mut out);
        }

        out.push_str("}\n");

        out
    }
}

type Key = u8;
fn main() {
    let mut b = BTreeMap::<Key, i32>::new();

    let mut t = AvlTreeMap::<Key, i32>::new();
    unsafe { SAVE_TARGET = T(&t as *const _) };

    // std::fs::remove_dir_all("./output").unwrap();
    // std::fs::create_dir("./output").unwrap();

    let mut rng = rand::rngs::SmallRng::seed_from_u64(114);

    let mut last = Instant::now();
    for i in 0..=10_000_000 {
        if i % 100 == 0 && last.elapsed().as_secs_f32() > 1.0 {
            println!("{i}, {}", b.len());
            last = Instant::now();
        }

        if rng.random_bool(0.5) {
            let (k, v): (Key, i32) = rng.random();
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

    save("out");
    // t.remove(&116);
    // save(&t, "out2");
}

struct T(*const AvlTreeMap<u8, i32>);
unsafe impl Sync for T {}

static mut SAVE_TARGET: T = T(std::ptr::null());
static SAVE_COUNT: AtomicUsize = AtomicUsize::new(0);

fn save(suffix: &str) {
    let c = SAVE_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let name = format!("out_{c}_{suffix}");

    let fname = format!("output/{name}.dot");
    std::fs::write(&fname, (unsafe { &*SAVE_TARGET.0 }).to_dot())
        .unwrap();
    let status = std::process::Command::new("dot")
        .args([&fname, "-Tjpg", &format!("-ooutput/{name}.jpg")])
        .status()
        .unwrap();
    assert!(status.success(), "{status:?}");
    std::fs::remove_file(&fname).unwrap();
}
