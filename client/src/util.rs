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
