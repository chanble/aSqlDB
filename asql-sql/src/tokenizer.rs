use std::collections::HashSet;

use crate::db_type::DatabaseType;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Function,
    String,
    Number,
    Identifier,
    Operator,
    Punctuation,
    Comment,
    Variable,
    Whitespace,
}

#[derive(Debug, Clone)]
pub struct Span {
    pub kind: TokenKind,
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn text<'a>(&self, sql: &'a str) -> &'a str {
        &sql[self.start..self.end]
    }
}

struct LexerState<'a> {
    sql: &'a str,
    chars: Vec<(usize, char)>,
    pos: usize,
    keywords: HashSet<String>,
    functions: HashSet<String>,
}

impl<'a> LexerState<'a> {
    fn new(sql: &'a str, keywords: HashSet<String>, functions: HashSet<String>) -> Self {
        let chars: Vec<(usize, char)> = sql.char_indices().collect();
        Self {
            sql,
            chars,
            pos: 0,
            keywords,
            functions,
        }
    }

    fn peek(&self) -> Option<(usize, char)> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<(usize, char)> {
        let c = self.chars.get(self.pos).copied();
        self.pos += 1;
        c
    }

    fn peek_at(&self, offset: usize) -> Option<(usize, char)> {
        self.chars.get(self.pos + offset).copied()
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn emit(&self, start: usize, end: usize, kind: TokenKind) -> Span {
        Span { kind, start, end }
    }

    fn read_word(&mut self, start: usize) -> usize {
        while let Some((_, ch)) = self.peek() {
            if ch.is_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }
        self.chars
            .get(self.pos - 1)
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(start)
    }

    fn read_number(&mut self, start: usize) -> usize {
        let mut has_dot = false;
        while let Some((_, ch)) = self.peek() {
            if ch == '.' && !has_dot {
                if let Some((_, next)) = self.peek_at(1) {
                    if next.is_ascii_digit() {
                        has_dot = true;
                        self.advance();
                        continue;
                    }
                }
                break;
            } else if ch.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        self.chars
            .get(self.pos - 1)
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(start)
    }

    fn read_single_quoted_string(&mut self, _start: usize) -> usize {
        while let Some((_, ch)) = self.advance() {
            if ch == '\'' {
                if let Some((_, '\'')) = self.peek() {
                    self.advance();
                    continue;
                }
                break;
            }
        }
        self.chars
            .get(self.pos.saturating_sub(1))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(self.sql.len())
    }

    fn read_double_quoted_string(&mut self, _start: usize) -> usize {
        while let Some((_, ch)) = self.advance() {
            if ch == '"' {
                break;
            }
        }
        self.chars
            .get(self.pos.saturating_sub(1))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(self.sql.len())
    }

    fn read_backtick_quoted(&mut self, _start: usize) -> usize {
        while let Some((_, ch)) = self.advance() {
            if ch == '`' {
                break;
            }
        }
        self.chars
            .get(self.pos.saturating_sub(1))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(self.sql.len())
    }

    fn read_line_comment(&mut self, start: usize) -> usize {
        while let Some((_, ch)) = self.peek() {
            if ch == '\n' {
                break;
            }
            self.advance();
        }
        if self.pos > 0 {
            self.chars
                .get(self.pos - 1)
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(self.sql.len())
        } else {
            start
        }
    }

    fn read_block_comment(&mut self, _start: usize) -> usize {
        while let Some((_, ch)) = self.advance() {
            if ch == '*' {
                if let Some((_, '/')) = self.peek() {
                    self.advance();
                    break;
                }
            }
        }
        self.chars
            .get(self.pos.saturating_sub(1))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(self.sql.len())
    }

    fn read_whitespace(&mut self, start: usize) -> usize {
        while let Some((_, ch)) = self.peek() {
            if ch.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
        self.chars
            .get(self.pos.saturating_sub(1))
            .map(|(i, c)| i + c.len_utf8())
            .unwrap_or(start)
    }

    fn classify_word(&self, text: &str) -> TokenKind {
        let upper = text.to_uppercase();
        if self.functions.contains(&upper) {
            TokenKind::Function
        } else if self.keywords.contains(&upper) {
            TokenKind::Keyword
        } else {
            TokenKind::Identifier
        }
    }
}

pub fn tokenize_sql(sql: &str, db_type: DatabaseType) -> Vec<Span> {
    let keywords: HashSet<String> = db_type
        .highlight_keywords()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let functions: HashSet<String> = db_type
        .highlight_functions()
        .iter()
        .map(|s| s.to_string())
        .collect();
    tokenize_sql_with(sql, &keywords, &functions)
}

pub fn tokenize_sql_with(
    sql: &str,
    keywords: &HashSet<String>,
    functions: &HashSet<String>,
) -> Vec<Span> {
    let mut state = LexerState::new(sql, keywords.clone(), functions.clone());
    let mut spans = Vec::new();

    while !state.is_eof() {
        let (start, ch) = state.advance().unwrap();

        match ch {
            '\'' => {
                let end = state.read_single_quoted_string(start);
                spans.push(state.emit(start, end, TokenKind::String));
            }
            '"' => {
                let end = state.read_double_quoted_string(start);
                spans.push(state.emit(start, end, TokenKind::String));
            }
            '`' => {
                let end = state.read_backtick_quoted(start);
                spans.push(state.emit(start, end, TokenKind::Identifier));
            }
            '/' => {
                if let Some((_, '*')) = state.peek() {
                    state.advance();
                    let end = state.read_block_comment(start);
                    spans.push(state.emit(start, end, TokenKind::Comment));
                } else {
                    let end = state
                        .chars
                        .get(state.pos.saturating_sub(1))
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(start);
                    spans.push(state.emit(start, end, TokenKind::Operator));
                }
            }
            '-' => {
                if let Some((_, '-')) = state.peek() {
                    state.advance();
                    let end = state.read_line_comment(start);
                    spans.push(state.emit(start, end, TokenKind::Comment));
                } else {
                    let end = state
                        .chars
                        .get(state.pos.saturating_sub(1))
                        .map(|(i, c)| i + c.len_utf8())
                        .unwrap_or(start);
                    spans.push(state.emit(start, end, TokenKind::Operator));
                }
            }
            '#' => {
                let end = state.read_line_comment(start);
                spans.push(state.emit(start, end, TokenKind::Comment));
            }
            '@' => {
                let end = state.read_word(start);
                spans.push(state.emit(start, end, TokenKind::Variable));
            }
            '=' | '!' | '<' | '>' | '|' => {
                if let Some((_, next)) = state.peek() {
                    if next == '=' || (ch == '|' && next == '|') || (ch == '<' && next == '>') {
                        state.advance();
                    }
                }
                let end = state
                    .chars
                    .get(state.pos.saturating_sub(1))
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(start);
                spans.push(state.emit(start, end, TokenKind::Operator));
            }
            '+' | '*' | '%' => {
                let end = state
                    .chars
                    .get(state.pos.saturating_sub(1))
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(start);
                spans.push(state.emit(start, end, TokenKind::Operator));
            }
            '(' | ')' | ',' | ';' | '.' => {
                let end = state
                    .chars
                    .get(state.pos.saturating_sub(1))
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(start);
                spans.push(state.emit(start, end, TokenKind::Punctuation));
            }
            _ if ch.is_ascii_digit() => {
                let end = state.read_number(start);
                spans.push(state.emit(start, end, TokenKind::Number));
            }
            _ if ch.is_alphabetic() || ch == '_' => {
                let end = state.read_word(start);
                let text = &sql[start..end];
                let kind = state.classify_word(text);
                spans.push(state.emit(start, end, kind));
            }
            _ if ch.is_whitespace() => {
                let end = state.read_whitespace(start);
                spans.push(state.emit(start, end, TokenKind::Whitespace));
            }
            _ => {
                let end = state
                    .chars
                    .get(state.pos.saturating_sub(1))
                    .map(|(i, c)| i + c.len_utf8())
                    .unwrap_or(start);
                spans.push(state.emit(start, end, TokenKind::Punctuation));
            }
        }
    }

    spans
}
