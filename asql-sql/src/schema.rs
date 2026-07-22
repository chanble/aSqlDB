use std::collections::HashMap;

use async_trait::async_trait;

use crate::provider::SchemaProvider;

#[derive(Debug, Clone)]
pub struct Column {
    pub name: String,
    pub data_type: String,
    pub comment: String,
}

#[derive(Debug, Clone)]
pub struct Table {
    pub name: String,
    pub columns: Vec<Column>,
    pub comment: String,
}

impl Table {
    pub fn column_names(&self) -> Vec<&str> {
        self.columns.iter().map(|c| c.name.as_str()).collect()
    }
}

#[derive(Debug, Clone, Default)]
pub struct DatabaseSchema {
    tables: HashMap<String, Table>,
}

impl DatabaseSchema {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_table(&mut self, table: Table) {
        self.tables.insert(table.name.clone(), table);
    }

    pub fn get_table(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }

    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }

    pub fn tables(&self) -> &HashMap<String, Table> {
        &self.tables
    }

    /// 更新指定表的字段列表（用于惰性加载字段补全）
    pub fn set_table_columns(&mut self, name: &str, columns: Vec<Column>) {
        if let Some(table) = self.tables.get_mut(name) {
            table.columns = columns;
        }
    }
}

#[async_trait]
impl SchemaProvider for DatabaseSchema {
    async fn table_names(&self, prefix: &str, limit: usize) -> Vec<String> {
        let prefix_lower = prefix.to_lowercase();
        self.tables
            .keys()
            .filter(|n| prefix.is_empty() || n.to_lowercase().starts_with(&prefix_lower))
            .take(limit)
            .cloned()
            .collect()
    }

    async fn columns(&self, table: &str) -> Vec<(String, String)> {
        self.tables
            .get(table)
            .map(|t| {
                t.columns
                    .iter()
                    .map(|c| (c.name.clone(), c.data_type.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    async fn columns_for(&self, tables: &[String]) -> Vec<(String, String)> {
        let mut seen = std::collections::HashSet::new();
        let mut out = Vec::new();
        for t in tables {
            for (name, ty) in self.columns(t).await {
                if seen.insert(name.clone()) {
                    out.push((name, ty));
                }
            }
        }
        out
    }
}
