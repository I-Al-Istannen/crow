use console::style;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Default)]
pub struct StyledText(String);

impl Display for StyledText {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl StyledText {
    pub fn append<T: Display>(mut self, text: T) -> Self {
        self.0.push_str(&text.to_string());
        self
    }
}

pub fn st<T: Display>(start: T) -> StyledText {
    StyledText(start.to_string())
}

pub fn color_diff<T: Display>(diff: T) -> String {
    let diff = diff
        .to_string()
        .lines()
        .map(|line| {
            if line.starts_with("-") {
                format!("{}", style(line).red().bright())
            } else if line.starts_with("+") {
                format!("{}", style(line).green().bright())
            } else if line.starts_with("@") {
                format!("{}", style(line).magenta())
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<String>>();

    diff.join("\n")
}
