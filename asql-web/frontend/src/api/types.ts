export interface SelectColumnBody {
  name: string
  func?: string
}

export interface WhereConditionBody {
  column: string
  operator: string
  value: string
}

export interface OrderByBody {
  column: string
  desc: boolean
}

export interface ColumnDefBody {
  name: string
  type: string
  length?: string
  options?: string
  nullable: boolean
  auto_increment: boolean
  primary_key?: boolean
  default_value?: string
  comment?: string
}

export interface ChangeColumnBody {
  old_name: string
  new_def: ColumnDefBody
}

export interface AddIndexBody {
  name: string
  index_type: string
  columns: string[]
}

export interface AlterTableBody {
  rename_table?: string
  engine?: string
  collation?: string
  comment?: string
  add_columns: ColumnDefBody[]
  modify_columns: ColumnDefBody[]
  change_columns: ChangeColumnBody[]
  drop_columns: string[]
  add_indexes: AddIndexBody[]
  drop_indexes: string[]
}

export interface CreateIndexBody {
  name: string
  index_type: string
  columns: Array<{ name: string; prefix_len?: number }>
  method?: string
}

export interface GrantBody {
  privileges: string[]
  on: string
  host?: string
  with_grant_option: boolean
}

export interface RevokeBody {
  privileges: string[]
  on: string
  host?: string
}
