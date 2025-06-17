use crate::types::{ExecutionExitStatus, FinishedTestSummary};
use evalexpr::{
    ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Node, Operator, Value,
};
use serde::Serialize;
use snafu::{ResultExt, Whatever};
use std::collections::HashMap;

#[derive(Debug, Default)]
struct CategoryInfo {
    pub passed_tests: usize,
    pub total_tests: usize,
}

impl CategoryInfo {
    fn update(&mut self, test: &FinishedTestSummary) {
        self.total_tests += 1;
        if test.output == ExecutionExitStatus::Success {
            self.passed_tests += 1;
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct GradingPoints {
    pub points: f64,
    pub formula: String,
}

impl GradingPoints {
    pub fn new(points: f64, formula: String) -> Self {
        Self { points, formula }
    }
}

/// Evaluates the grading formula for a task based on the provided tests.
pub fn get_points_for_task(
    formula: &Node,
    tests: &[FinishedTestSummary],
) -> Result<GradingPoints, Whatever> {
    if tests.is_empty() {
        return Ok(GradingPoints::new(0.0, formula_to_string(formula)));
    }

    let categories: HashMap<String, CategoryInfo> =
        tests.iter().fold(HashMap::new(), |mut acc, test| {
            let Some(category) = &test.category else {
                return acc;
            };
            if let Some(provisional_category) = &test.provisional_for_category {
                // This test was provisional for its category, so we do not count it.
                if provisional_category == category {
                    return acc;
                }
            }

            acc.entry(category.clone()).or_default().update(test);
            acc
        });

    get_points(formula, categories)
}

fn get_points(
    formula: &Node,
    categories: HashMap<String, CategoryInfo>,
) -> Result<GradingPoints, Whatever> {
    let mut context: HashMapContext<DefaultNumericTypes> = HashMapContext::new();

    for (name, info) in &categories {
        let slug = slugify(name);
        context
            .set_value(
                format!("passed_{slug}"),
                Value::Float(info.passed_tests as f64),
            )
            .whatever_context(format!("failed to set passed tests for category '{name}'"))?;
        context
            .set_value(
                format!("total_{slug}"),
                Value::Float(info.total_tests as f64),
            )
            .whatever_context(format!("failed to set total tests for category '{name}'"))?;
    }

    let res = formula
        .eval_with_context(&context)
        .whatever_context("failed to evaluate grading formula")?;

    let res = res
        .as_float()
        .whatever_context("grading formula did not return a float")?;

    // Let's not return negative points
    Ok(GradingPoints::new(res.max(0.0), formula_to_string(formula)))
}

fn slugify(name: &str) -> String {
    let name = name
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    name.trim_matches('_').to_string()
}

fn formula_to_string(formula: &Node) -> String {
    let operator = formula.operator();
    let children = formula
        .children()
        .iter()
        .map(formula_to_string)
        .collect::<Vec<_>>();

    match operator {
        Operator::RootNode => children.join(" ").to_string(),
        Operator::Add
        | Operator::Sub
        | Operator::Mul
        | Operator::Div
        | Operator::Mod
        | Operator::Eq
        | Operator::Gt
        | Operator::Lt
        | Operator::Geq
        | Operator::Leq
        | Operator::Neq
        | Operator::And
        | Operator::Or
        | Operator::Assign
        | Operator::AddAssign
        | Operator::SubAssign
        | Operator::MulAssign
        | Operator::DivAssign
        | Operator::ModAssign
        | Operator::ExpAssign
        | Operator::AndAssign
        | Operator::OrAssign => {
            format!("({})", children.join(&format!(" {operator} ")))
        }
        Operator::Neg => format!("(-{})", children.join(" ")),
        Operator::Exp => format!("({}^{})", children[0], children[1]),
        Operator::Not => format!("(!{})", children.join(" ")),
        Operator::Tuple => format!("({})", children.join(", ")),
        Operator::Chain => format!("({})", children.join(" ")),
        Operator::Const { .. } => operator.to_string(),
        Operator::VariableIdentifierWrite { .. } => format!("{operator} = {}", children.join(", ")),
        Operator::VariableIdentifierRead { .. } => format!("{operator}{}", children.join(" ")),
        Operator::FunctionIdentifier { .. } => format!("{operator}{}", children.join(", ")),
    }
}
