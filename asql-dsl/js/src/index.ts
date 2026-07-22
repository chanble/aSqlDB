import type { InitOutput } from '../pkg/aqua_dsl.js'
import init, * as wasm from '../pkg/aqua_dsl.js'

let ready: Promise<InitOutput> | null = null

export function initDsl(): Promise<InitOutput> {
  if (!ready) {
    ready = init()
  }
  return ready!
}

// ── Types ────────────────────────────────────────────────────────────

export interface WhereCondition {
  column: string
  operator: string
  value: string
  logical?: 'AND' | 'OR'
}

export interface SelectColumn {
  name: string
  func?: string | null
}

export interface OrderByClause {
  column: string
  desc?: boolean
}

export interface UpdateSet {
  column: string
  value: string
}

export interface ColumnDef {
  name: string
  type: string
  length?: string | null
  options?: string
  nullable?: boolean
  auto_increment?: boolean
  default?: string | null
  comment?: string | null
  on_update?: boolean
}

export interface AlterColumnDef {
  name: string
  type: string
  length?: string | null
  options?: string
  nullable?: boolean
  auto_increment?: boolean
  default?: string | null
  comment?: string | null
  on_update?: boolean
}

export type IndexType = 'Index' | 'Unique' | 'Primary' | 'Fulltext' | 'Spatial'

export type AlterAction =
  | { type: 'AddColumn'; column: AlterColumnDef }
  | { type: 'ModifyColumn'; column: AlterColumnDef }
  | { type: 'ChangeColumn'; old_name: string; column: AlterColumnDef }
  | { type: 'DropColumn'; name: string }
  | { type: 'RenameTable'; new_name: string }
  | { type: 'AddPrimaryKey'; columns: string[] }
  | { type: 'DropPrimaryKey' }
  | { type: 'AddIndex'; name: string; index_type: IndexType; columns: string[] }
  | { type: 'DropIndex'; name: string }

// ── Helpers ─────────────────────────────────────────────────────────

const DIALECT = 'mysql'

function json(data: unknown): string {
  return JSON.stringify(data)
}

// ── Builder functions ────────────────────────────────────────────────

export function buildSelect(params: {
  table: string
  columns?: SelectColumn[]
  where?: WhereCondition[]
  orderBy?: OrderByClause[]
  limit?: number
  offset?: number
}): string {
  return wasm.select_sql(json({ ...params, dialect: DIALECT }))
}

export function buildCount(params: {
  table: string
  where?: WhereCondition[]
}): string {
  return wasm.count_sql(json({ ...params, dialect: DIALECT }))
}

export function buildInsert(params: {
  table: string
  columns: string[]
  values: string[][]
}): string {
  return wasm.insert_sql(json({ ...params, dialect: DIALECT }))
}

export function buildUpdate(params: {
  table: string
  sets: UpdateSet[]
  where?: WhereCondition[]
  limit?: number
}): string {
  return wasm.update_sql(json({ ...params, dialect: DIALECT }))
}

export function buildDelete(params: {
  table: string
  where?: WhereCondition[]
  limit?: number
}): string {
  return wasm.delete_sql(json({ ...params, dialect: DIALECT }))
}

export function buildCreateTable(params: {
  table: string
  columns: ColumnDef[]
  engine?: string
  collation?: string
}): string {
  return wasm.create_table_sql(json({ ...params, dialect: DIALECT }))
}

export function buildAlterTable(params: {
  table: string
  actions: AlterAction[]
}): string[] {
  const result = wasm.alter_table_sql(json({ ...params, dialect: DIALECT }))
  return JSON.parse(result)
}

export function buildCreateIndex(params: {
  table: string
  name?: string
  index_type?: IndexType
  columns: string[]
}): string {
  return wasm.create_index_sql(json({ ...params, dialect: DIALECT }))
}

export function buildCreateDatabase(params: {
  name: string
  collation?: string
}): string {
  return wasm.create_database_sql(json({ ...params, dialect: DIALECT }))
}

export function buildDropTable(table: string, ifExists?: boolean, dialect?: string): string {
  return wasm.drop_table_sql(table, ifExists ?? false, dialect ?? DIALECT)
}

/**
 * Build SQL to truncate a table.
 * @param table Table name.
 * @param dialect Optional dialect name (default 'mysql').
 * @returns SQL string like "TRUNCATE TABLE `logs`" or "DELETE FROM `logs`" (SQLite)
 */
export function buildTruncate(table: string, dialect?: string): string {
  return wasm.truncate_sql(table, dialect ?? DIALECT)
}

/**
 * Build SQL to drop a database.
 * @param name Database name.
 * @param ifExists Optional IF EXISTS clause (default false).
 * @param dialect Optional dialect name (default 'mysql').
 * @returns SQL string like "DROP DATABASE `mydb`"
 */
export function buildDropDatabase(name: string, ifExists?: boolean, dialect?: string): string {
  return wasm.drop_database_sql(name, ifExists ?? false, dialect ?? DIALECT)
}

/**
 * Build SQL to drop an index.
 * @param name Index name.
 * @param table Table name.
 * @param ifExists Optional IF EXISTS clause (default false).
 * @param dialect Optional dialect name (default 'mysql').
 * @returns SQL string like "DROP INDEX `idx_name` ON `users`"
 */
export function buildDropIndex(name: string, table: string, ifExists?: boolean, dialect?: string): string {
  return wasm.drop_index_sql(name, table, ifExists ?? false, dialect ?? DIALECT)
}

export function buildCreateUser(params: {
  username: string
  host?: string
  password: string
}): string {
  return wasm.create_user_sql(json({ ...params, dialect: DIALECT }))
}

export function buildGrant(params: {
  privileges: string[]
  on: string
  to: string
  host?: string
}): string {
  return wasm.grant_sql(json({ ...params, dialect: DIALECT }))
}

// ── Introspection ────────────────────────────────────────────────────

export function buildListTables(database?: string): string {
  return wasm.list_tables_sql(DIALECT, database ?? null)
}

/**
 * Build SQL to get metadata for a single table, including its comment.
 * @param database Database name.
 * @param table Table name.
 * @param dialect Optional dialect name (default 'mysql').
 * @returns SQL string like "SELECT TABLE_NAME, TABLE_COMMENT, ... FROM information_schema.tables WHERE ..."
 */
export function buildTableInfo(database: string, table: string, dialect?: string): string {
  return wasm.table_info_sql(dialect ?? DIALECT, database, table)
}

export function buildListDatabases(): string {
  return wasm.list_databases_sql(DIALECT)
}

/**
 * Build SQL to query the current database name.
 * @param dialect Optional dialect name (default 'mysql').
 * @returns SQL string like "SELECT DATABASE() as db"
 */
export function buildCurrentDatabase(dialect?: string): string {
  return wasm.current_database_sql(dialect ?? DIALECT)
}

/**
 * Build SQL to query the database server version.
 * @param dialect Optional dialect name (default 'mysql').
 * @returns SQL string like "SELECT VERSION() as version"
 */
export function buildVersion(dialect?: string): string {
  return wasm.version_sql(dialect ?? DIALECT)
}

export function buildShowColumns(table: string, database?: string): string {
  return wasm.show_columns_sql(DIALECT, table, database ?? null)
}

export function buildShowCreateTable(table: string): string {
  return wasm.show_create_table_sql(DIALECT, table)
}

export function buildListIndexes(table: string): string {
  return wasm.list_indexes_sql(DIALECT, table)
}
