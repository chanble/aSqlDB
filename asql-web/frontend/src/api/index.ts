import type {
  Connection,
  ConnectionParams,
  SystemInfo,
  ExecutionResult,
  SelectResult,
  DatabaseInfo,
  TableInfo,
  TableSize,
  ColumnDef,
  IndexDetail,
  UserInfo,
  ProcessInfo,
  VariableInfo,
  DdlSummary,
  ModifySummary,
  QueryResultItem,
} from '../types'
import type {
  SelectColumnBody,
  WhereConditionBody,
  OrderByBody,
  ColumnDefBody,
  AlterTableBody,
  AddIndexBody,
  CreateIndexBody,
  GrantBody,
  RevokeBody,
} from './types'

const BASE = '/api'

async function request<T>(path: string, options?: RequestInit): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    headers: { 'Content-Type': 'application/json' },
    ...options,
  })
  if (!res.ok) {
    const body = await res.json().catch(() => ({}))
    throw new Error(body.error || `HTTP ${res.status}`)
  }
  return res.json()
}

function enc(s: string): string {
  return encodeURIComponent(s)
}

export const api = {
  health: () => request<{ status: string }>('/health'),

  listSystems: () => request<SystemInfo[]>('/systems'),

  listConnections: () => request<Connection[]>('/connections'),

  addConnection: (config: { name: string; params: ConnectionParams }) =>
    request<{ success: boolean }>('/connections', {
      method: 'POST',
      body: JSON.stringify(config),
    }),

  removeConnection: (name: string) =>
    request<{ success: boolean }>(`/connections/${enc(name)}`, {
      method: 'DELETE',
    }),

  testConnection: (name: string) =>
    request<{ success: boolean }>(`/connections/${enc(name)}/test`, {
      method: 'POST',
    }),

  executeQuery: (connection: string, sql: string, stopOnError = false) =>
    request<QueryResultItem[]>(`/connections/${enc(connection)}/query`, {
      method: 'POST',
      body: JSON.stringify({ sql, stop_on_error: stopOnError }),
    }),

  listDatabases: (connection: string) =>
    request<ExecutionResult<DatabaseInfo[]>>(`/connections/${enc(connection)}/databases`),

  createDatabase: (connection: string, name: string, characterSet?: string, collation?: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/databases`, {
      method: 'POST',
      body: JSON.stringify({ name, character_set: characterSet, collation }),
    }),

  dropDatabase: (connection: string, db: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/databases/${enc(db)}`, {
      method: 'DELETE',
    }),

  alterDatabase: (connection: string, db: string, characterSet?: string, collation?: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/databases/${enc(db)}`, {
      method: 'PATCH',
      body: JSON.stringify({ character_set: characterSet, collation }),
    }),

  currentDatabase: (connection: string) =>
    request<ExecutionResult<string>>(`/connections/${enc(connection)}/current-database`),

  useDatabase: (connection: string, database: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/use-database`, {
      method: 'POST',
      body: JSON.stringify({ database }),
    }),

  listTables: (connection: string, db: string, tableName?: string, exact = false) =>
    request<ExecutionResult<TableInfo[]>>(
      `/connections/${enc(connection)}/databases/${enc(db)}/tables?${new URLSearchParams(tableName ? { table_name: tableName, exact: String(exact) } : { exact: String(exact) })}`,
    ),

  tableCount: (connection: string, db: string) =>
    request<ExecutionResult<number>>(`/connections/${enc(connection)}/databases/${enc(db)}/tables/count`),

  tableSizes: (connection: string, db: string) =>
    request<ExecutionResult<TableSize[]>>(`/connections/${enc(connection)}/databases/${enc(db)}/tables/sizes`),

  dropTables: (connection: string, db: string, tables: string[]) =>
    request<Array<{ success: boolean; data?: ExecutionResult<DdlSummary>; error?: string }>>(
      `/connections/${enc(connection)}/databases/${enc(db)}/tables/drop`,
      { method: 'POST', body: JSON.stringify({ tables }) },
    ),

  truncateTables: (connection: string, db: string, tables: string[]) =>
    request<Array<{ success: boolean; data?: ExecutionResult<DdlSummary>; error?: string }>>(
      `/connections/${enc(connection)}/databases/${enc(db)}/tables/truncate`,
      { method: 'POST', body: JSON.stringify({ tables }) },
    ),

  repairTables: (connection: string, db: string, tables: string[]) =>
    request<Array<{ success: boolean; data?: ExecutionResult<DdlSummary>; error?: string }>>(
      `/connections/${enc(connection)}/databases/${enc(db)}/tables/repair`,
      { method: 'POST', body: JSON.stringify({ tables }) },
    ),

  optimizeTables: (connection: string, db: string, tables: string[]) =>
    request<Array<{ success: boolean; data?: ExecutionResult<DdlSummary>; error?: string }>>(
      `/connections/${enc(connection)}/databases/${enc(db)}/tables/optimize`,
      { method: 'POST', body: JSON.stringify({ tables }) },
    ),

  analyzeTables: (connection: string, db: string, tables: string[]) =>
    request<Array<{ success: boolean; data?: ExecutionResult<DdlSummary>; error?: string }>>(
      `/connections/${enc(connection)}/databases/${enc(db)}/tables/analyze`,
      { method: 'POST', body: JSON.stringify({ tables }) },
    ),

  checkTables: (connection: string, db: string, tables: string[]) =>
    request<Array<{ success: boolean; data?: ExecutionResult<DdlSummary>; error?: string }>>(
      `/connections/${enc(connection)}/databases/${enc(db)}/tables/check`,
      { method: 'POST', body: JSON.stringify({ tables }) },
    ),

  createTable: (connection: string, db: string, body: { table: string; columns: ColumnDefBody[]; engine?: string; collation?: string; comment?: string }) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/databases/${enc(db)}/tables`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  tableInfo: (connection: string, table: string, database?: string) =>
    request<ExecutionResult<TableInfo[]>>(
      `/connections/${enc(connection)}/tables/${enc(table)}/info?${database ? `database=${enc(database)}` : ''}`,
    ),

  dropTable: (connection: string, table: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/tables/${enc(table)}`, {
      method: 'DELETE',
    }),

  truncateTable: (connection: string, table: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/tables/${enc(table)}/truncate`, {
      method: 'POST',
    }),

  alterTable: (connection: string, table: string, body: AlterTableBody) =>
    request<ExecutionResult<DdlSummary>[]>(`/connections/${enc(connection)}/tables/${enc(table)}/alter`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  showColumns: (connection: string, table: string, database?: string) =>
    request<ExecutionResult<ColumnDef[]>>(
      `/connections/${enc(connection)}/tables/${enc(table)}/columns?${database ? `database=${enc(database)}` : ''}`,
    ),

  showCreateTable: (connection: string, table: string) =>
    request<ExecutionResult<string>>(`/connections/${enc(connection)}/tables/${enc(table)}/create-table`),

  getFunctions: (connection: string) =>
    request<Array<{ name: string; category: string }>>(
      `/connections/${enc(connection)}/functions`,
    ),

  selectData: (connection: string, table: string, body: {
    columns: SelectColumnBody[]
    where_conditions: WhereConditionBody[]
    order_by: OrderByBody[]
    limit?: number
    offset?: number
  }) =>
    request<ExecutionResult<SelectResult>>(`/connections/${enc(connection)}/tables/${enc(table)}/select`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  countData: (connection: string, table: string, whereConditions: WhereConditionBody[]) =>
    request<ExecutionResult<number>>(`/connections/${enc(connection)}/tables/${enc(table)}/count`, {
      method: 'POST',
      body: JSON.stringify({ where_conditions: whereConditions }),
    }),

  insertData: (connection: string, table: string, body: { columns: string[]; values: string[][] }) =>
    request<ExecutionResult<ModifySummary>>(`/connections/${enc(connection)}/tables/${enc(table)}/insert`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  updateData: (connection: string, table: string, body: {
    sets: Array<{ column: string; value: string }>
    where_conditions: WhereConditionBody[]
    limit?: number
  }) =>
    request<ExecutionResult<ModifySummary>>(`/connections/${enc(connection)}/tables/${enc(table)}/data`, {
      method: 'PATCH',
      body: JSON.stringify(body),
    }),

  deleteData: (connection: string, table: string, body: {
    where_conditions: WhereConditionBody[]
    limit?: number
  }) =>
    request<ExecutionResult<ModifySummary>>(`/connections/${enc(connection)}/tables/${enc(table)}/data`, {
      method: 'DELETE',
      body: JSON.stringify(body),
    }),

  listIndexes: (connection: string, table: string) =>
    request<ExecutionResult<IndexDetail[]>>(`/connections/${enc(connection)}/tables/${enc(table)}/indexes`),

  createIndex: (connection: string, table: string, body: CreateIndexBody) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/tables/${enc(table)}/indexes`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  dropIndex: (connection: string, table: string, index: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/tables/${enc(table)}/indexes/${enc(index)}`, {
      method: 'DELETE',
    }),

  listUsers: (connection: string) =>
    request<ExecutionResult<UserInfo[]>>(`/connections/${enc(connection)}/users`),

  userInfo: (connection: string, username: string, host?: string) =>
    request<ExecutionResult<UserInfo[]>>(
      `/connections/${enc(connection)}/users/${enc(username)}?${host ? `host=${enc(host)}` : ''}`,
    ),

  createUser: (connection: string, body: { username: string; password?: string; host?: string }) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/users`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  alterUser: (connection: string, username: string, body: { password: string; host?: string }) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/users/${enc(username)}`, {
      method: 'PUT',
      body: JSON.stringify(body),
    }),

  dropUser: (connection: string, username: string, host?: string) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/users/${enc(username)}`, {
      method: 'DELETE',
      body: JSON.stringify({ host }),
    }),

  renameUser: (connection: string, username: string, body: { new_username: string; new_host?: string }) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/users/${enc(username)}/rename`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  grant: (connection: string, username: string, body: GrantBody) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/users/${enc(username)}/grant`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  revoke: (connection: string, username: string, body: RevokeBody) =>
    request<ExecutionResult<DdlSummary>>(`/connections/${enc(connection)}/users/${enc(username)}/revoke`, {
      method: 'POST',
      body: JSON.stringify(body),
    }),

  processList: (connection: string) =>
    request<ExecutionResult<ProcessInfo[]>>(`/connections/${enc(connection)}/processes`),

  killProcesses: (connection: string, pids: string[]) =>
    request<Array<{ success: boolean; data?: Record<string, any>; error?: string }>>(
      `/connections/${enc(connection)}/processes/kill`,
      { method: 'POST', body: JSON.stringify({ pids }) },
    ),

  variables: (connection: string) =>
    request<ExecutionResult<VariableInfo[]>>(`/connections/${enc(connection)}/variables`),

  status: (connection: string) =>
    request<ExecutionResult<VariableInfo[]>>(`/connections/${enc(connection)}/status`),

  version: (connection: string) =>
    request<ExecutionResult<string>>(`/connections/${enc(connection)}/version`),

  pingConnection: (connection: string) =>
    request<{ success: boolean }>(`/connections/${enc(connection)}/ping`, {
      method: 'POST',
    }),

  columnTypes: (connection: string) =>
    request<Array<{ category: string; types: string[] }>>(`/connections/${enc(connection)}/column-types`),

  charsets: (connection: string) =>
    request<Array<{ charset: string; collations: string[] }>>(`/connections/${enc(connection)}/charsets`),

  importPreview: (connection: string, filePath: string) =>
    request<{ total_lines: number; file_size: number; head: string; tail: string; omitted: number }>(
      `/connections/${enc(connection)}/import/preview`,
      { method: 'POST', body: JSON.stringify({ file_path: filePath }) },
    ),

  importServerFile: (
    connection: string,
    filePath: string,
    options?: { stopOnError?: boolean; database?: string; singleTransaction?: boolean; fileName?: string },
  ) =>
    request<{ task_id: string }>(`/connections/${enc(connection)}/import`, {
      method: 'POST',
      body: JSON.stringify({
        file_path: filePath,
        stop_on_error: options?.stopOnError ?? false,
        single_transaction: options?.singleTransaction ?? false,
        database: options?.database,
        file_name: options?.fileName,
      }),
    }),

  importStatus: (taskId: string) =>
    request<{
      id: string
      status: 'running' | 'completed' | 'failed' | 'cancelled'
      total: number
      current: number
      succeeded: number
      failed: number
      duration_ms: number
      error: string | null
      errors: Array<{ index: number; error: string }>
      connection: string
      database: string | null
      file_name: string
      file_path: string
      total_lines: number
      file_size: number
      preview_head: string
      preview_tail: string
      preview_omitted: number
      stop_on_error: boolean
      single_transaction: boolean
      created_at: number
      finished_at: number | null
    }>(`/import/tasks/${enc(taskId)}`),

  listImportTasks: () =>
    request<{
      tasks: Array<{
        id: string
        connection: string
        database: string | null
        file_name: string
        file_path: string
        status: 'running' | 'completed' | 'failed' | 'cancelled'
        total: number
        total_lines: number
        file_size: number
        current: number
        succeeded: number
        failed: number
        error_count: number
        duration_ms: number
        created_at: number
        finished_at: number | null
        stop_on_error: boolean
        single_transaction: boolean
      }>
    }>(`/import/tasks`),

  cancelImport: (taskId: string) =>
    request<{ ok: boolean }>(`/import/tasks/${enc(taskId)}/cancel`, { method: 'POST' }),

  uploadImportFile: async (connection: string, file: File) => {
    const formData = new FormData()
    formData.append('file', file)
    const res = await fetch(`${BASE}/connections/${enc(connection)}/import/upload`, {
      method: 'POST',
      body: formData,
    })
    if (!res.ok) {
      const body = await res.json().catch(() => ({}))
      throw new Error(body.error || `HTTP ${res.status}`)
    }
    return res.json() as Promise<{
      file_path: string
      original_name: string
      file_size: number
    }>
  },
}
