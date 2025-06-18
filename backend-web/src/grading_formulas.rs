use crate::config::{TestCategory, TestConfig};
use crate::error::WebError;
use crate::types::{ExecutionExitStatus, FinishedTestSummary};
use evalexpr::{
    ContextWithMutableVariables, DefaultNumericTypes, HashMapContext, Node, Operator, Value,
};
use serde::Serialize;
use snafu::{location, Report, ResultExt, Whatever};
use std::borrow::Borrow;
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
    tests: &[impl Borrow<FinishedTestSummary>],
) -> Result<GradingPoints, Whatever> {
    if tests.is_empty() {
        return Ok(GradingPoints::new(0.0, formula_to_string(formula)));
    }

    let categories: HashMap<String, CategoryInfo> =
        tests.iter().fold(HashMap::new(), |mut acc, test| {
            let Some(category) = &test.borrow().category else {
                return acc;
            };
            if let Some(provisional_category) = &test.borrow().provisional_for_category {
                // This test was provisional for its category, so we do not count it.
                if provisional_category == category {
                    return acc;
                }
            }

            acc.entry(category.clone())
                .or_default()
                .update(test.borrow());
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
        .or(res.as_int().map(|i| i as f64))
        .whatever_context("grading formula did not return am int/float")?;

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

/// This answers the question: "This task was submitted in category X, how many points does it get?"
/// To answer this, we need to look at all test results for this task and then only count those
/// where the test category is either from *earlier* or the test is not provisional.
pub fn get_grading_points_for_task(
    test_config: &TestConfig,
    category_name: &str,
    meta: &TestCategory,
    summaries: &[impl Borrow<FinishedTestSummary>],
) -> Result<Option<GradingPoints>, WebError> {
    // If we have no grading formula, we cannot calculate points
    let Some(grading_formula) = &meta.grading_formula else {
        return Ok(None);
    };

    let summaries = test_config.get_counting_tests(category_name, summaries);

    // Fetch grading points for the task using the grading formula
    let points = get_points_for_task(grading_formula, &summaries);
    let points = match points {
        Ok(points) => Some(points),
        Err(e) => {
            return Err(WebError::internal_error(
                Report::from_error(&e).to_string(),
                location!(),
            ));
        }
    };

    Ok(points)
}
