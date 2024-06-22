use std::fmt::Display;

#[warn(missing_debug_implementations)]

fn main() {
    let input_1 = r#"test source"#;
    let input_2 = r#"test target"#;

    fn print_diff<T: PartialEq + Display>(diff: Vec<EditInfo<'_, T>>) {
        for edit in diff {
            match edit {
                EditInfo::Unchange { source } => {
                    print!("{source}");
                }
                EditInfo::Delete { source } => {
                    print!("[91m{source}[m");
                }
                EditInfo::Insert { target } => {
                    print!("[92m{target}[m");
                }
                EditInfo::Substitute { target, .. } => {
                    print!("[93m{target}[m");
                }
            }
        }
    }

    let differ = Differ::new(input_1.chars().collect(), input_2.chars().collect(), true);
    print_diff(differ.gen_diff());

    println!("\n==================================================================\n");

    let differ = Differ::new(input_1.lines().collect(), input_2.lines().collect(), false);
    for edit in differ.gen_diff() {
        match edit {
            EditInfo::Unchange { source } => {
                print!("{source}");
            }
            EditInfo::Delete { source } => {
                print!("[91m{source}[m");
            }
            EditInfo::Insert { target } => {
                print!("[92m{target}[m");
            }
            EditInfo::Substitute { source, target } => {
                let differ = Differ::new(source.chars().collect(), target.chars().collect(), true);
                let diff = differ.gen_diff();

                print_diff(diff)
            }
        }
        println!();
    }
}

pub struct Differ<T: PartialEq> {
    disable_substitution: bool,

    source: Vec<T>,
    target: Vec<T>,
    height: usize,

    distance_matrix: Vec<(u64, EditType)>,
}

impl<T: PartialEq> Differ<T> {
    pub fn new(source: Vec<T>, target: Vec<T>, disable_substitution: bool) -> Self {
        let height = target.len() + 1;
        let distance_matrix = vec![(0, EditType::N); (source.len() + 1) * height];
        let mut this = Self {
            disable_substitution,

            source,
            target,
            height,

            distance_matrix,
        };
        this.calc_distance();
        this
    }

    fn coord_to_idx(&self, idx_source: usize, idx_target: usize) -> usize {
        idx_source * self.height + idx_target
    }

    fn get(&self, idx_source: usize, idx_target: usize) -> (u64, EditType) {
        self.distance_matrix[self.coord_to_idx(idx_source, idx_target)]
    }

    fn set(&mut self, idx_source: usize, idx_target: usize, new: (u64, EditType)) {
        let idx = self.coord_to_idx(idx_source, idx_target);
        self.distance_matrix[idx] = new;
    }

    // https://en.wikipedia.org/wiki/Wagner%E2%80%93Fischer_algorithm
    fn calc_distance(&mut self) {
        for idx_source in 1..=self.source.len() {
            self.set(idx_source, 0, (idx_source as u64, EditType::D));
        }

        for idx_target in 1..=self.target.len() {
            self.set(0, idx_target, (idx_target as u64, EditType::I));
        }

        for idx_source in 1..=self.source.len() {
            for idx_target in 1..=self.target.len() {
                let deletion = (self.get(idx_source - 1, idx_target).0 + 1, EditType::D);
                let insertion = (self.get(idx_source, idx_target - 1).0 + 1, EditType::I);
                let substitution = self.get(idx_source - 1, idx_target - 1).0;
                let substitution = if self.source[idx_source - 1] == self.target[idx_target - 1] {
                    (substitution, EditType::N)
                } else {
                    (
                        substitution + if self.disable_substitution { 114514 } else { 1 },
                        EditType::S,
                    )
                };

                let result = if deletion.0 <= insertion.0 && deletion.0 <= substitution.0 {
                    deletion
                } else if insertion.0 <= deletion.0 && insertion.0 <= substitution.0 {
                    insertion
                } else {
                    substitution
                };

                self.set(idx_source, idx_target, result);
            }
        }
    }

    pub fn gen_diff(&self) -> Vec<EditInfo<'_, T>> {
        let mut cur_pos = (self.source.len(), self.target.len());
        let mut diff = vec![];

        while cur_pos.0 > 0 || cur_pos.1 > 0 {
            let cur = self.get(cur_pos.0, cur_pos.1);

            let source = &self.source[cur_pos.0.saturating_sub(1)];
            let target = &self.target[cur_pos.1.saturating_sub(1)];

            let v = match cur.1 {
                EditType::N => EditInfo::Unchange { source },
                EditType::D => EditInfo::Delete { source },
                EditType::I => EditInfo::Insert { target },
                EditType::S => EditInfo::Substitute { source, target },
            };
            diff.push(v);

            cur_pos = match cur.1 {
                EditType::N | EditType::S => (cur_pos.0 - 1, cur_pos.1 - 1),
                EditType::D => (cur_pos.0 - 1, cur_pos.1),
                EditType::I => (cur_pos.0, cur_pos.1 - 1),
            };
            // println!("{cur_pos:?} {cur:?}");
        }

        diff.reverse();

        diff
    }

    pub fn step_count(&self) -> u64 {
        self.distance_matrix[self.distance_matrix.len() - 1].0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditType {
    N, // None
    D, // Delete
    I, // Insert
    S, // Substitute
}

#[derive(Debug, Clone, Copy)]
pub enum EditInfo<'value, T> {
    Unchange {
        source: &'value T,
    },
    Delete {
        source: &'value T,
    },
    Insert {
        target: &'value T,
    },
    Substitute {
        source: &'value T,
        target: &'value T,
    },
}

impl<'value, T> EditInfo<'value, T> {
    pub fn to_num(&self) -> u64 {
        match self {
            EditInfo::Unchange { .. } => 0,
            EditInfo::Delete { .. } => 1,
            EditInfo::Insert { .. } => 2,
            EditInfo::Substitute { .. } => 3,
        }
    }
}
