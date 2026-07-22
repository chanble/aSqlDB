<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'

import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import SearchableSelect from '../components/SearchableSelect.vue'
import { api } from '../api'

const route = useRoute()
const router = useRouter()
const { t } = useI18n()

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)
const table = computed(() => route.params.table as string)

const columns = ref<any[]>([])
const indexes = ref<any[]>([])
const loading = ref(true)
const saving = ref(false)

interface IndexRow {
  type: string
  cols: { name: string }[]
  name: string
}

interface IndexSnapshot {
  name: string
  type: string
  cols: string[]
}

const originalIndexes = ref<IndexSnapshot[]>([])
const indexRows = ref<IndexRow[]>([])

onMounted(async () => {
  await loadInfo()
  loading.value = false
})

async function loadInfo() {
  try {
    // Get columns
    const colResult = await api.showColumns(connection.value, table.value, database.value)
    if (colResult?.data) {
      columns.value = colResult.data
    }

    // Get indexes
    const idxResult = await api.listIndexes(connection.value, table.value)
    indexes.value = idxResult?.data || []

    // Build snapshots and populate editable rows
    const snapMap = new Map<string, { type: string; cols: string[] }>()
    for (const idx of indexes.value) {
      const key = idx.key_name
      if (!snapMap.has(key)) {
        const type = idx.key_name === 'PRIMARY' ? 'PRIMARY' : idx.non_unique ? 'INDEX' : 'UNIQUE'
        snapMap.set(key, { type, cols: [] })
      }
      snapMap.get(key)!.cols.push(idx.column_name)
    }
    originalIndexes.value = Array.from(snapMap.entries()).map(([name, val]) => ({
      name, type: val.type, cols: val.cols,
    }))
    indexRows.value = originalIndexes.value.map(orig => ({
      name: orig.name,
      type: orig.type,
      cols: orig.cols.map(c => ({ name: c })),
    }))
    if (indexRows.value.length === 0) {
      indexRows.value.push({ type: '', cols: [{ name: '' }], name: '' })
    }
  } catch { /* ignore */ }
}

function addIndexRow() {
  indexRows.value.push({ type: '', cols: [{ name: '' }], name: '' })
}

function addColToIndex(i: number) {
  indexRows.value[i].cols.push({ name: '' })
}

function removeColFromIndex(i: number, j: number) {
  indexRows.value[i].cols.splice(j, 1)
}

function removeIndexRow(i: number) {
  indexRows.value.splice(i, 1)
}

async function save() {
  saving.value = true
  try {
    const currentNames = new Set<string>()
    const rowsToProcess: { name: string; type: string; cols: string[] }[] = []

    for (const row of indexRows.value) {
      if (!row.name && row.cols.every(c => !c.name)) continue
      const validCols = row.cols.filter(c => c.name)
      if (validCols.length === 0) continue

      const effectiveName = row.type === 'PRIMARY' ? 'PRIMARY' : row.name
      currentNames.add(effectiveName)
      rowsToProcess.push({
        name: effectiveName,
        type: row.type || 'INDEX',
        cols: validCols.map(c => c.name),
      })
    }

    // Drop removed indexes
    for (const orig of originalIndexes.value) {
      if (!currentNames.has(orig.name)) {
        await api.dropIndex(connection.value, table.value, orig.name)
      }
    }

    // Create or recreate changed indexes
    for (const row of rowsToProcess) {
      const orig = originalIndexes.value.find(o => o.name === row.name)
      const unchanged = orig && orig.type === row.type &&
        orig.cols.length === row.cols.length &&
        orig.cols.every((c, i) => c === row.cols[i])
      if (unchanged) continue

      if (orig) {
        await api.dropIndex(connection.value, table.value, row.name)
      }

      const cols = row.cols.map(c => ({ name: c }))
      if (row.type === 'PRIMARY') {
        await api.createIndex(connection.value, table.value, { name: 'PRIMARY', index_type: 'PRIMARY', columns: cols })
      } else if (row.type === 'UNIQUE') {
        await api.createIndex(connection.value, table.value, { name: row.name, index_type: 'UNIQUE', columns: cols })
      } else {
        await api.createIndex(connection.value, table.value, { name: row.name, index_type: 'INDEX', columns: cols })
      }
    }

    alert(t('indexes.saved'))
    await loadInfo()
  } catch (e: any) {
    alert(t('indexes.error', { msg: e.message || e }))
  }
  saving.value = false
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('indexes.title') }}: {{ table }}</div>
    <div class="page-content">
      <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <template v-else>
        <table style="width:auto">
          <thead>
            <tr>
              <th>{{ $t('indexes.type') }}</th>
              <th>{{ $t('indexes.columns') }}</th>
              <th>{{ $t('indexes.name') }}</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(row, i) in indexRows" :key="i">
              <td>
                <SearchableSelect v-model="row.type" :options="[{value:'',label:'-'},{value:'PRIMARY',label:'PRIMARY'},{value:'INDEX',label:'INDEX'},{value:'UNIQUE',label:'UNIQUE'},{value:'FULLTEXT',label:'FULLTEXT'},{value:'SPATIAL',label:'SPATIAL'}]" style="width:100px" />
              </td>
              <td>
                <span v-for="(col, j) in row.cols" :key="j" style="display:inline-flex;gap:2px;margin-right:4px">
                  <SearchableSelect v-model="col.name" :options="[{value:'',label:'-'},...columns.map(c=>({value:c.name,label:c.name}))]" style="width:100px" />
                </span>
                <button @click="addColToIndex(i)" style="font-size:11px">+</button>
              </td>
              <td>
                <input v-model="row.name" type="text" style="width:150px">
              </td>
              <td>
                <button @click="removeIndexRow(i)" style="font-size:11px">×</button>
              </td>
            </tr>
          </tbody>
        </table>

        <div style="margin-top:8px">
          <button @click="addIndexRow" style="font-size:11px">{{ $t('indexes.addIndex') }}</button>
        </div>

        <div style="margin-top:12px">
          <button @click="save" :disabled="saving">{{ $t('indexes.save') }}</button>
        </div>

      </template>
    </div>
  </div>
</template>
