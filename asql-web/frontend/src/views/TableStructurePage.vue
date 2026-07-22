<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import SearchableSelect from '../components/SearchableSelect.vue'
import SqlFeedback from '../components/SqlFeedback.vue'
import type { SqlFeedbackItem } from '../components/SqlFeedback.vue'
import { api } from '../api'

const { t } = useI18n()

const route = useRoute()
const router = useRouter()

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)
const table = computed(() => route.params.table as string)

const columns = ref<any[]>([])
const indexes = ref<any[]>([])
const groupedIndexes = computed(() => {
  const map = new Map<string, { key_name: string; column_names: string }>()
  for (const idx of indexes.value) {
    const key = idx.key_name
    if (!map.has(key)) {
      map.set(key, { key_name: key, column_names: '' })
    }
    const group = map.get(key)!
    group.column_names += (group.column_names ? ', ' : '') + idx.column_name
  }
  return Array.from(map.values())
})
const loading = ref(false)
const comment = ref('')
const engine = ref('')
const collation = ref('')
const feedbackItems = ref<SqlFeedbackItem[]>([])

function pad(n: number): string {
  return n.toString().padStart(2, '0')
}

onMounted(async () => {
  await loadAll()
})

async function loadAll() {
  if (!connection.value || !database.value || !table.value) return
  loading.value = true
  try {
    const tbl = table.value
    const showResult = await api.showColumns(connection.value, tbl, database.value)
    if (showResult.data) {
      columns.value = showResult.data.map((r: any) => ({
        Field: r.name,
        Type: r.data_type || '',
        Null: r.nullable,
        Key: r.key || '',
        Default: r.default ?? null,
        Extra: buildExtra(r.extra),
        Comment: r.comment || '',
      }))
    }

function buildExtra(extra: { auto_increment: boolean; on_update: boolean }): string {
  const parts: string[] = []
  if (extra.auto_increment) parts.push('auto_increment')
  if (extra.on_update) parts.push('on update CURRENT_TIMESTAMP')
  return parts.join('  ')
}

    // Get table info
    const infoResult = await api.tableInfo(connection.value, tbl, database.value)
    if (infoResult.data && infoResult.data.length > 0) {
      comment.value = infoResult.data[0].table_comment ?? ''
      engine.value = infoResult.data[0].engine ?? ''
      collation.value = infoResult.data[0].table_collation ?? ''
    }
  } catch { /* ignore */ }

  try {
    const idxResult = await api.listIndexes(connection.value, table.value)
    indexes.value = idxResult.data || []
  } catch { /* ignore */ }

  loading.value = false
}

function goBack() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}`)
}

function goData() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(table.value)}`)
}

function goIndexes() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(table.value)}/indexes`)
}

function goAlterTable() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(table.value)}/alter`)
}

function goSql() {
  router.push(`/query/${encodeURIComponent(connection.value)}`)
}

// --- Column Operations ---
async function dropColumn(colName: string) {
  if (!confirm(`Drop column \`${colName}\`?`)) return
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  try {
    const result = await api.alterTable(connection.value, table.value, {
      add_columns: [], modify_columns: [], change_columns: [],
      drop_columns: [colName],
      add_indexes: [], drop_indexes: [],
    })
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: true,
      sql: result[0]?.sql || `ALTER TABLE \`${table.value}\` DROP COLUMN \`${colName}\``,
      duration: result[0]?.duration_ms ?? 0,
      time,
    }]
    await loadAll()
  } catch (e: any) {
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: false,
      sql: e.message || String(e),
      duration: 0,
      time,
    }]
  }
}

// --- Drop Table ---
async function doDropTable() {
  if (!confirm(`Drop table \`${table.value}\`?`)) return
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  try {
    const result = await api.dropTable(connection.value, table.value)
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: true,
      sql: result.sql || `DROP TABLE \`${table.value}\``,
      duration: result.duration_ms ?? 0,
      time,
    }]
    goBack()
  } catch (e: any) {
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: false,
      sql: e.message || String(e),
      duration: 0,
      time,
    }]
  }
}

// --- Truncate Table ---
async function doTruncateTable() {
  if (!confirm(`Truncate table \`${table.value}\`?`)) return
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  try {
    const result = await api.truncateTable(connection.value, table.value)
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: true,
      sql: result.sql || `TRUNCATE TABLE \`${table.value}\``,
      duration: result.duration_ms ?? 0,
      time,
    }]
  } catch (e: any) {
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: false,
      sql: e.message || String(e),
      duration: 0,
      time,
    }]
  }
}

// --- Index Management ---
const showIndexes = ref(false)
const newIndexName = ref('')
const newIndexCols = ref('')
const newIndexType = ref('INDEX')

async function doCreateIndex() {
  if (!newIndexName.value || !newIndexCols.value) return
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  try {
    const columns = newIndexCols.value.split(',').map(c => ({ name: c.trim() }))
    const result = await api.createIndex(connection.value, table.value, {
      name: newIndexName.value,
      index_type: newIndexType.value,
      columns,
    })
    showIndexes.value = false
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: true,
      sql: result.sql || '',
      duration: result.duration_ms ?? 0,
      time,
    }]
    await loadAll()
  } catch (e: any) {
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: false,
      sql: e.message || String(e),
      duration: 0,
      time,
    }]
  }
}

async function doDropIndex(name: string) {
  if (!confirm(`Drop index \`${name}\`?`)) return
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  try {
    const result = await api.dropIndex(connection.value, table.value, name)
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: true,
      sql: result.sql || '',
      duration: result.duration_ms ?? 0,
      time,
    }]
    await loadAll()
  } catch (e: any) {
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: false,
      sql: e.message || String(e),
      duration: 0,
      time,
    }]
  }
}

function formatIndexType(idx: Record<string, any>): string {
  return idx.index_type || 'BTREE'
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('tableStructure.title') }}: {{ table }}</div>
    <div class="page-content">
      <div class="top-actions">
        <a href="#" @click.prevent="goData">{{ $t('tableStructure.selectData') }}</a>
        <a href="#" class="active">{{ $t('tableStructure.showStructure') }}</a>
        <a href="#" @click.prevent="goAlterTable">{{ $t('tableStructure.alterTable') }}</a>
        <a href="#">{{ $t('tableStructure.newItem') }}</a>
      </div>

      <SqlFeedback :items="feedbackItems" :connection="connection" @remove="(i: number) => feedbackItems.splice(i, 1)" />

      <div v-if="comment || engine || collation" style="margin-bottom:8px;font-size:13px">
        <span v-if="comment">Comment: {{ comment }}</span>
        <span v-if="engine" style="margin-left:16px">Engine: {{ engine }}</span>
        <span v-if="collation" style="margin-left:16px">Collation: {{ collation }}</span>
      </div>

      <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <template v-else>
        <table style="width:auto;margin-bottom:16px">
          <thead>
            <tr>
              <th>{{ $t('tableStructure.column') }}</th>
              <th>{{ $t('tableStructure.type') }}</th>
              <th>{{ $t('tableStructure.comment') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="col in columns" :key="col.Field">
              <td>
                <strong>{{ col.Field }}</strong>
                <a href="#" @click.prevent="dropColumn(col.Field)" style="margin-left:8px;font-size:11px">{{ $t('tableStructure.drop') }}</a>
              </td>
              <td>{{ col.Type }}{{ col.Null ? '' : ' NOT NULL' }}{{ col.Key === 'PRI' ? ' PRIMARY KEY' : '' }}{{ col.Extra }}</td>
              <td>{{ col.Comment }}</td>
            </tr>
          </tbody>
        </table>

        <button @click="goAlterTable">{{ $t('tableStructure.addColumn') }}</button>

        <h3 style="font-size:14px;font-weight:normal;margin:16px 0 8px">{{ $t('tableStructure.indexes') }}</h3>
        <table v-if="indexes.length" style="width:auto;margin-bottom:8px">
          <thead>
            <tr>
              <th>{{ $t('tableStructure.name') }}</th>
              <th>{{ $t('tableStructure.columns') }}</th>
              <th>{{ $t('tableStructure.type') }}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(idx, i) in groupedIndexes" :key="i">
              <td><strong>{{ idx.key_name }}</strong></td>
              <td>{{ idx.column_names }}</td>
              <td>{{ formatIndexType(idx) }}</td>
              <td><a href="#" @click.prevent="doDropIndex(idx.key_name)">{{ $t('tableStructure.drop') }}</a></td>
            </tr>
          </tbody>
        </table>
        <div v-else style="font-size:13px;color:#999;margin-bottom:8px">{{ $t('tableStructure.noIndexes') }}</div>
        <a href="#" @click.prevent="goIndexes">{{ $t('tableStructure.alterIndexes') }}</a>

        <div style="margin-top:16px">
          <button class="danger" @click="doTruncateTable">{{ $t('tableStructure.truncate') }}</button>
          <button class="danger" style="margin-left:8px" @click="doDropTable">{{ $t('tableStructure.dropTable') }}</button>
        </div>
      </template>
      <!-- Add Index Modal -->
      <div v-if="showIndexes" style="position:fixed;top:0;left:0;right:0;bottom:0;background:rgba(0,0,0,0.4);z-index:200;display:flex;align-items:center;justify-content:center" @click.self="showIndexes = false">
        <div style="background:#fff;padding:16px;border:1px solid #999;min-width:400px">
          <h3 style="font-size:14px;font-weight:normal;margin-bottom:12px">{{ $t('tableStructure.addIndex') }}</h3>
          <table class="form-table">
            <tbody>
              <tr>
                <th>{{ $t('tableStructure.name') }}</th>
                <td><input v-model="newIndexName" type="text" style="width:200px"></td>
              </tr>
              <tr>
                <th>{{ $t('tableStructure.columns') }}</th>
                <td><input v-model="newIndexCols" type="text" placeholder="col1, col2" style="width:200px"></td>
              </tr>
              <tr>
                <th>{{ $t('tableStructure.type') }}</th>
                <td>
                  <SearchableSelect v-model="newIndexType" :options="[{value:'INDEX',label:'INDEX'},{value:'UNIQUE',label:'UNIQUE'}]" />
                </td>
              </tr>
            </tbody>
          </table>
          <div style="margin-top:12px">
            <button @click="doCreateIndex">{{ $t('tableStructure.save') }}</button>
            <button style="margin-left:8px" @click="showIndexes = false">{{ $t('tableStructure.cancel') }}</button>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
