export interface Connection {
  name: string
  url: string
  db_type: string
}

export type ConnectionParams =
  | { type: 'MySql'; host: string; port: number; user: string; password: string | null; database: string | null }
  | { type: 'Postgres'; host: string; port: number; user: string; password: string | null; database: string | null }
  | { type: 'Sqlite'; path: string }

export interface SystemInfo {
  value: string
  label: string
  defaultPort: number | null
  params: ConnectionParams
}

export interface ExecutionResult<T> {
  sql: string
  duration_ms: number
  data: T
}

export interface DatabaseInfo {
  name: string
  collation: string
}

export interface TableInfo {
  table_name: string
  table_comment: string | null
  engine: string | null
  table_collation: string | null
  table_rows: number
  table_size: number
  data_length: number | null
  index_length: number | null
  data_free: number | null
  auto_increment: number | null
}

export interface TableSize {
  table_name: string
  size_bytes: number
}

export interface ColumnDef {
  name: string
  col_type: any
  data_type: string
  nullable: boolean | null
  default: string | null
  comment: string | null
  extra: {
    auto_increment: boolean
    on_update: boolean
  }
  collation: string | null
  key: string | null
}

export interface SelectResult {
  columns: ColumnDef[]
  rows: Array<Record<string, any>>
}

export interface IndexDetail {
  key_name: string
  column_name: string
  non_unique: boolean
  seq_in_index: number
  index_type: string
}

export interface UserInfo {
  user: string
  host: string | null
}

export interface ProcessInfo {
  id: number
  user: string
  host: string
  db: string | null
  command: string
  time: number
  state: string | null
  info: string | null
}

export interface VariableInfo {
  name: string
  value: string
}

export interface DdlSummary {
}

export interface ModifySummary {
  rows_affected: number
  last_insert_id: number | null
}

export interface QueryResultItem {
  success: boolean
  data?: Record<string, any>
  error?: string
}
