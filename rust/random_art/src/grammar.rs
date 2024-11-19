use std::{collections::HashMap, ops::RangeInclusive};

use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::node::Node;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grammer {
    pub rules: HashMap<RuleId, Rule>,
}

impl Grammer {
    /// #Panics:
    ///     panic if has invalid rule reference or empty rule
    pub fn gen(
        &self,
        rng: &mut impl Rng,
        initial_rule: RuleId,
        depth: i64,
    ) -> Node {
        let rule =
            self.rules.get(&initial_rule).expect("expect rule exists");

        let item = if depth <= 0 {
            &rule.items[0]
        } else {
            let total_weight =
                rule.items.iter().map(|it| it.weight).sum::<f64>();
            let mut rnd = rng.gen_range(0.0..total_weight);
            let mut target = None;

            for item in rule.items.iter() {
                rnd -= item.weight;
                if rnd < 0.0 {
                    target = Some(item);
                    break;
                }
            }

            target.expect("expect non empty rule")
        };

        let mut depth = depth;
        if depth >= 0 && rng.gen_range(0.0..1.0) <= 0.5 {
            depth -= 1;
        }

        item.a.expand(rng, &|rng, id| self.gen(rng, id, depth - 1))
    }
}

#[derive(
    Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize, Deserialize,
)]
pub struct RuleId(pub u64);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub items: Vec<RuleItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleItem {
    pub a: RuleNode,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuleNode {
    Rule(RuleId),

    X,
    Y,
    Const(f64),
    Lit(RangeInclusive<f64>),
    Rgb(Box<RuleNode>, Box<RuleNode>, Box<RuleNode>),

    Add(Box<RuleNode>, Box<RuleNode>),
    Sub(Box<RuleNode>, Box<RuleNode>),
    Mul(Box<RuleNode>, Box<RuleNode>),
    Div(Box<RuleNode>, Box<RuleNode>),
    Mod(Box<RuleNode>, Box<RuleNode>),
    Pow(Box<RuleNode>, Box<RuleNode>),
    Sin(Box<RuleNode>),
    Cos(Box<RuleNode>),
    Exp(Box<RuleNode>),
    Sqrt(Box<RuleNode>),
    Mix(Box<RuleNode>, Box<RuleNode>, Box<RuleNode>, Box<RuleNode>),
}

impl RuleNode {
    pub fn expand<R: Rng>(
        &self,
        rng: &mut R,
        fetch_rule: &impl Fn(&mut R, RuleId) -> Node,
    ) -> Node {
        let mut expand =
            |node: &RuleNode| Box::new(node.expand(rng, fetch_rule));

        match self {
            RuleNode::Rule(rule_id) => fetch_rule(rng, *rule_id),
            RuleNode::X => Node::X,
            RuleNode::Y => Node::Y,
            RuleNode::Const(x) => Node::Lit(*x),
            RuleNode::Lit(range) => {
                Node::Lit(rng.gen_range(range.clone()))
            }
            RuleNode::Rgb(r, g, b) => {
                Node::Rgb(expand(r), expand(g), expand(b))
            }
            RuleNode::Add(lhs, rhs) => {
                Node::Add(expand(lhs), expand(rhs))
            }
            RuleNode::Sub(lhs, rhs) => {
                Node::Sub(expand(lhs), expand(rhs))
            }
            RuleNode::Mul(lhs, rhs) => {
                Node::Mul(expand(lhs), expand(rhs))
            }
            RuleNode::Div(lhs, rhs) => {
                Node::Div(expand(lhs), expand(rhs))
            }
            RuleNode::Mod(lhs, rhs) => {
                Node::Mod(expand(lhs), expand(rhs))
            }
            RuleNode::Pow(lhs, rhs) => {
                Node::Pow(expand(lhs), expand(rhs))
            }
            RuleNode::Sin(x) => Node::Sin(expand(x)),
            RuleNode::Cos(x) => Node::Cos(expand(x)),
            RuleNode::Exp(x) => Node::Exp(expand(x)),
            RuleNode::Sqrt(x) => Node::Sqrt(expand(x)),
            RuleNode::Mix(a, b, c, d) => {
                Node::Mix(expand(a), expand(b), expand(c), expand(d))
            }
        }
    }
}
