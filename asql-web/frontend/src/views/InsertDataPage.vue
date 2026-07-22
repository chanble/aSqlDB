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

const loading = ref(true)
const saving = ref(false)

interface ColumnField {
  name: string
  type: string
  isAutoIncrement: boolean
  nullable: boolean
  sqlExpr: string
  value: string
}

const fields = ref<ColumnField[]>([])

const sqlOptions: Record<string, { value: string; label: string }[]> = {}

function getSqlOptions(col: { type: string; nullable: boolean }) {
  const opts: { value: string; label: string }[] = [{ value: '', label: '' }]
  opts.push({ value: 'NULL', label: t('insertData.null') })
  const typeUpper = col.type.toUpperCase()
  if (typeUpper.includes('DATETIME') || typeUpper.includes('TIMESTAMP')) {
    opts.push({ value: 'now', label: 'now' })
  }
  opts.push({ value: 'SQL', label: 'SQL' })
  return opts
}

onMounted(async () => {
  await loadColumns()
})

async function loadColumns() {
  loading.value = true
  try {
    const result = await api.showColumns(connection.value, table.value, database.value)
    fields.value = (result.data || []).map((r: any) => ({
      name: r.name || '',
      type: r.data_type || 'text',
      isAutoIncrement: r.extra.auto_increment,
      nullable: r.nullable === true,
      sqlExpr: '',
      value: '',
    }))
  } catch { /* ignore */ }
  loading.value = false
}

async function insert(andNext: boolean) {
  if (fields.value.length === 0) return
  saving.value = true
  try {
    const cols: string[] = []
    const vals: string[] = []
    for (const f of fields.value) {
      if (f.isAutoIncrement && !f.sqlExpr && !f.value) continue
      cols.push(f.name)
      if (f.sqlExpr === 'NULL') {
        vals.push('NULL')
      } else if (f.sqlExpr === 'now') {
        vals.push('NOW()')
      } else if (f.sqlExpr === 'SQL' && f.value) {
        vals.push(f.value)
      } else {
        vals.push(f.value || '')
      }
    }
    await api.insertData(connection.value, table.value, {
      columns: cols,
      values: [vals],
    })
    alert('Insert successful')
    if (andNext) {
      for (const f of fields.value) {
        if (!f.isAutoIncrement) {
          f.value = ''
          f.sqlExpr = ''
        }
      }
    } else {
      router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(table.value)}`)
    }
  } catch (e: any) {
    alert(`Error: ${e.message || e}`)
  }
  saving.value = false
}

function save() {
  insert(false)
}

function saveAndInsertNext() {
  insert(true)
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('insertData.title') }}: {{ table }}</div>
    <div class="page-content">
      <div v-if="loading" style="padding:20px;text-align:center;color:#999">Loading...</div>

      <template v-else>
        <table class="insert-table">
          <tbody>
            <tr v-for="f in fields" :key="f.name">
              <td class="col-name">{{ f.name }}</td>
              <td class="col-action">
                <template v-if="f.isAutoIncrement">
                  <span class="ai-label">Auto Increment</span>
                </template>
                <template v-else>
                  <SearchableSelect v-model="f.sqlExpr" :options="getSqlOptions(f)" size="small" style="width:70px" />
                </template>
              </td>
              <td class="col-value">
                <template v-if="!f.isAutoIncrement">
                  <input v-model="f.value" type="text" class="value-input" :disabled="f.sqlExpr === 'NULL'" :placeholder="f.type">
                </template>
              </td>
            </tr>
          </tbody>
        </table>

        <div style="margin-top:12px">
          <button @click="save" :disabled="saving">{{ $t('insertData.insert') }}</button>
          <button @click="saveAndInsertNext" :disabled="saving" style="margin-left:8px">Save and insert next</button>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.insert-table {
  border-collapse: collapse;
  width: auto;
}

.insert-table td {
  border: 1px solid #ccc;
  padding: 4px 8px;
  font-size: 13px;
}

.col-name {
  font-weight: bold;
  white-space: nowrap;
  background: #f8f8f8;
}

.col-action {
  white-space: nowrap;
}

.col-value {
  min-width: 200px;
}

.ai-label {
  color: #666;
  font-size: 12px;
}

.value-input {
  width: 100%;
  min-width: 180px;
  font-size: 13px;
  padding: 2px 4px;
  border: 1px solid #999;
  font-family: var(--adminer-font);
  box-sizing: border-box;
}

.value-input:disabled {
  background: #f0f0f0;
  color: #999;
}
</style>
