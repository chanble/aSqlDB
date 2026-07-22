use crate::dialect::Dialect;

/// Builder for column introspection queries across supported dialects
pub struct ColumnsIntrospection;

impl ColumnsIntrospection {
    /// Generate a query to list all columns for a given table, optionally scoped to a database/schema
    pub fn show_columns(dialect: &dyn Dialect, table: &str, database: Option<&str>) -> String {
        match dialect.name() {
            "MySQL" => {
                if let Some(db) = database {
                    format!(
                        "SHOW FULL COLUMNS FROM {}.{}",
                        dialect.quote_ident(db),
                        dialect.quote_ident(table)
                    )
                } else {
                    format!("SHOW FULL COLUMNS FROM {}", dialect.quote_ident(table))
                }
            }
            "PostgreSQL" => {
                let schema = match database {
                    Some(s) => s.replace('\'', "''"),
                    None => "public".to_string(),
                };
                let escaped_table = table.replace('\'', "''");
                format!(
                    "SELECT c.column_name, c.data_type, c.is_nullable, c.column_default, \
                     c.ordinal_position, c.collation_name, \
                     c.is_identity, c.identity_generation, \
                     pgd.description AS comment \
                     FROM information_schema.columns c \
                     LEFT JOIN pg_class pc \
                       ON pc.relname = c.table_name \
                       AND pc.relnamespace = (SELECT oid FROM pg_namespace WHERE nspname = '{schema}') \
                     LEFT JOIN pg_description pgd \
                       ON pgd.objoid = pc.oid \
                       AND pgd.objsubid = c.ordinal_position \
                     WHERE c.table_schema = '{schema}' \
                       AND c.table_name = '{escaped_table}' \
                     ORDER BY c.ordinal_position"
                )
            }
            "SQLite" => {
                format!(
                    "SELECT name AS column_name, type AS data_type, \
                     CASE WHEN NOT notnull THEN 'YES' ELSE 'NO' END AS is_nullable, \
                     dflt_value AS column_default, \
                     pk AS is_primary_key, \
                     collation AS collation_name \
                     FROM pragma_table_xinfo('{}') \
                     ORDER BY cid",
                    table.replace('\'', "''")
                )
            }
            _ => unreachable!(),
        }
    }

    pub fn show_create_table(dialect: &dyn Dialect, table: &str) -> String {
        match dialect.name() {
            "MySQL" => {
                format!("SHOW CREATE TABLE {}", dialect.quote_ident(table))
            }
            "PostgreSQL" => {
                format!(
                    "SELECT column_name || ' ' || data_type \
                     FROM information_schema.columns \
                     WHERE table_name = '{}' ORDER BY ordinal_position",
                    table.replace('\'', "''")
                )
            }
            "SQLite" => {
                format!(
                    "SELECT sql FROM sqlite_master WHERE type='table' AND name='{}'",
                    table.replace('\'', "''")
                )
            }
            _ => unreachable!(),
        }
    }
}
