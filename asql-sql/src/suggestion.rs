#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SuggestionKind {
    Keyword,
    Table,
    Column,
    Function,
    Alias,
}

#[derive(Debug, Clone)]
pub struct Suggestion {
    pub text: String,
    pub kind: SuggestionKind,
}

impl Suggestion {
    pub fn keyword(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SuggestionKind::Keyword,
        }
    }

    pub fn table(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SuggestionKind::Table,
        }
    }

    pub fn column(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SuggestionKind::Column,
        }
    }

    pub fn function(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SuggestionKind::Function,
        }
    }

    pub fn alias(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            kind: SuggestionKind::Alias,
        }
    }
}
