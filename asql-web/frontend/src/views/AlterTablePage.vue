<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import SearchableSelect from '../components/SearchableSelect.vue'
import { api } from '../api'

const route = useRoute()
const router = useRouter()

const { t } = useI18n()

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)
const table = computed(() => route.params.table as string)

const tableName = ref('')
const engine = ref('InnoDB')
const collation = ref('utf8mb4_general_ci')
const tableComment = ref('')
const saving = ref(false)
const loading = ref(true)

interface ColumnDef {
  name: string
  type: string
  length: string
  options: string
  nullable: boolean
  autoIncrement: boolean
  default: string
  comment: string
  originalName: string
  onUpdate: boolean
}

const columns = ref<ColumnDef[]>([])
const alterColumnOptions = computed(() => [
  { value: '', label: '-' },
  { value: 'unsigned', label: t('alterTable.unsigned') },
  { value: 'zerofill', label: t('alterTable.zerofill') },
  { value: 'unsigned zerofill', label: t('alterTable.unsigned') + ' ' + t('alterTable.zerofill') }
])

const typeGroups = ref<Array<{ category: string; types: string[] }>>([])
const collations = ref<Array<{ charset: string; collations: string[] }>>([])

const collationOptions = computed(() => {
  const opts: { value: string; label: string }[] = []
  for (const group of collations.value) {
    for (const item of group.collations) {
      opts.push({ value: item, label: item })
    }
  }
  return opts
})

onMounted(async () => {
  await loadTableInfo()
  await loadColumnTypes()
  await loadCollations()
})

async function loadTableInfo() {
  loading.value = true
  try {
    const infoResult = await api.tableInfo(connection.value, table.value, database.value)
    if (infoResult.data && infoResult.data.length > 0) {
      tableName.value = infoResult.data[0].table_name || table.value
      engine.value = infoResult.data[0].engine || 'InnoDB'
      collation.value = infoResult.data[0].table_collation || 'utf8mb4_general_ci'
      tableComment.value = infoResult.data[0].table_comment ?? ''
    }

    const colResult = await api.showColumns(connection.value, table.value, database.value)
    if (colResult.data) {
      columns.value = colResult.data.map((r: any) => {
        const typeMatch = (r.data_type || '').match(/^(\w+)(?:\((\d+)\))?/)
        const type = typeMatch ? typeMatch[1] : 'varchar'
        const length = typeMatch ? (typeMatch[2] || '') : ''
        const options = (r.data_type || '').toLowerCase().includes('unsigned') ? 'unsigned' : ''
        return {
          name: r.name || '',
          originalName: r.name || '',
          type,
          length,
          options,
          nullable: r.nullable === true,
          autoIncrement: r.extra.auto_increment,
          onUpdate: r.extra.on_update,
          default: r.default ?? '',
          comment: r.comment || '',
        }
      })
    }
  } catch { /* ignore */ }
  loading.value = false
}

async function loadColumnTypes() {
  try {
    typeGroups.value = await api.columnTypes(connection.value)
  } catch { /* ignore */ }
}

async function loadCollations() {
  try {
    collations.value = await api.charsets(connection.value)
  } catch { /* ignore */ }
}

function addColumn() {
  columns.value.push({
    name: '',
    originalName: '',
    type: 'varchar',
    length: '255',
    options: '',
    nullable: true,
    autoIncrement: false,
    onUpdate: false,
    default: '',
    comment: '',
  })
}

function removeColumn(i: number) {
  columns.value.splice(i, 1)
}

function moveColumn(i: number, dir: number) {
  const j = i + dir
  if (j < 0 || j >= columns.value.length) return
  const temp = columns.value[i]
  columns.value[i] = columns.value[j]
  columns.value[j] = temp
}

async function save() {
  if (!tableName.value.trim()) {
    alert('Table name is required')
    return
  }

  const body: Record<string, any> = {}

  if (tableName.value !== table.value) {
    body.rename_table = tableName.value
  }

  function calcAfterColumn(idx: number): string | undefined {
    for (let i = idx - 1; i >= 0; i--) {
      const prev = columns.value[i]
      if (prev.originalName) return prev.originalName
    }
    return 'FIRST'
  }

  const addColumns: any[] = []
  const modifyColumns: any[] = []
  const changeColumns: { old_name: string; new_def: any }[] = []

  for (let i = 0; i < columns.value.length; i++) {
    const col = columns.value[i]
    if (!col.name.trim()) continue

    const colDef: Record<string, any> = {
      name: col.name,
      type: col.type,
      length: col.length || undefined,
      options: col.options || undefined,
      nullable: col.nullable,
      auto_increment: col.autoIncrement,
      default_value: col.default || undefined,
      comment: col.comment || undefined,
      after_column: calcAfterColumn(i),
    }

    if (col.originalName && col.originalName !== col.name) {
      changeColumns.push({ old_name: col.originalName, new_def: colDef })
    } else if (col.originalName) {
      modifyColumns.push(colDef)
    } else {
      addColumns.push(colDef)
    }
  }

  if (addColumns.length > 0) body.add_columns = addColumns
  if (modifyColumns.length > 0) body.modify_columns = modifyColumns
  if (changeColumns.length > 0) body.change_columns = changeColumns

  if (engine.value) body.engine = engine.value
  if (collation.value) body.collation = collation.value
  if (tableComment.value) body.comment = tableComment.value

  if (Object.keys(body).length === 0) {
    alert('No changes to save')
    return
  }

  saving.value = true
  try {
    const results = await api.alterTable(connection.value, table.value, body as import('../api/types').AlterTableBody)
    const err = results.find(r => 'error' in r)
    if (err) {
      alert(`Error: ${err.error || 'Unknown error'}`)
      return
    }
    alert('Table altered successfully')
    router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(tableName.value)}/structure`)
  } catch (e: any) {
    alert(`Error: ${e.message || e}`)
  } finally {
    saving.value = false
  }
}

async function dropTable() {
  if (!confirm(`DROP TABLE \`${table.value}\`?`)) return
  try {
    await api.dropTable(connection.value, table.value)
    router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}`)
  } catch (e: any) {
    alert(`Error: ${e.message || e}`)
  }
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('alterTable.title') }}: {{ table }}</div>
    <div class="page-content">
      <div v-if="loading" style="padding:20px;text-align:center;color:#999">Loading...</div>

      <template v-else>
        <table class="form-table" style="margin-bottom:12px">
          <tbody>
            <tr>
              <th>{{ $t('alterTable.name') }}</th>
              <td>
                <input v-model="tableName" type="text" style="width:150px">
                <SearchableSelect v-model="engine" :options="[{value:'InnoDB',label:'InnoDB'},{value:'MyISAM',label:'MyISAM'},{value:'MEMORY',label:'MEMORY'},{value:'CSV',label:'CSV'}]" style="margin-left:4px;min-width:100px" />
                <SearchableSelect v-model="collation" :options="collationOptions" style="margin-left:4px;min-width:180px" />
                <input v-model="tableComment" type="text" style="width:150px;margin-left:4px" :placeholder="$t('alterTable.tableComment')">
              </td>
            </tr>
          </tbody>
        </table>

        <table style="width:auto">
          <thead>
            <tr>
              <th>{{ $t('alterTable.columnName') }}</th>
              <th>{{ $t('alterTable.dataType') }}</th>
              <th>{{ $t('alterTable.length') }}</th>
              <th>Options</th>
              <th>{{ $t('alterTable.nullable') }}</th>
              <th>{{ $t('alterTable.autoIncrement') }}</th>
              <th>{{ $t('alterTable.onUpdate') }}</th>
              <th>{{ $t('alterTable.defaultValue') }}</th>
              <th>{{ $t('alterTable.comment') }}</th>
              <th style="width:100px"></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(col, i) in columns" :key="i">
              <td><input v-model="col.name" type="text" style="width:150px"></td>
              <td>
                <SearchableSelect v-model="col.type" :options="typeGroups.flatMap(g=>g.types.map(t=>({value:t,label:t})))" style="width:100px" />
              </td>
              <td><input v-model="col.length" type="text" style="width:60px"></td>
              <td>
                <SearchableSelect v-model="col.options" :options="alterColumnOptions" style="width:100px" />
              </td>
              <td><input v-model="col.nullable" type="checkbox"></td>
              <td><input v-model="col.autoIncrement" type="checkbox"></td>
              <td><input v-model="col.onUpdate" type="checkbox"></td>
              <td><input v-model="col.default" type="text" style="width:100px"></td>
              <td><input v-model="col.comment" type="text" style="width:200px"></td>
              <td>
                <button @click="addColumn" :title="$t('alterTable.addColumn')">+</button>
                <button @click="moveColumn(i, -1)" :disabled="i === 0" title="Move up">↑</button>
                <button @click="moveColumn(i, 1)" :disabled="i === columns.length - 1" title="Move down">↓</button>
                <button @click="removeColumn(i)" :title="$t('alterTable.remove')">×</button>
              </td>
            </tr>
          </tbody>
        </table>

        <div style="margin-top:12px">
          <button @click="save" :disabled="saving">{{ $t('alterTable.save') }}</button>
          <button class="danger" style="margin-left:8px" @click="dropTable">{{ $t('alterTable.drop')}}</button>
        </div>

        <fieldset style="margin-top:16px;display:inline-block">
          <legend>Partition by</legend>
          <div class="fieldset-content">
            <span style="font-size:12px;color:#999">Not implemented</span>
          </div>
        </fieldset>
      </template>
    </div>
  </div>
</template>
