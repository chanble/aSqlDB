<script setup lang="ts">
import { ref, watch, computed, onMounted, nextTick } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { api } from '../api'
import SearchableSelect from '../components/SearchableSelect.vue'
import SqlFeedback from '../components/SqlFeedback.vue'
import type { SqlFeedbackItem } from '../components/SqlFeedback.vue'
import type { ColumnDef } from '../types'


const route = useRoute()
const router = useRouter()

const { t } = useI18n()

const columns = ref<{ name: string; data_type: string; key: string | null; comment: string | null }[]>([])
const allColumns = ref<ColumnDef[]>([])
const rows = ref<Record<string, any>[]>([])
const loading = ref(false)
const total = ref(0)
const page = ref(1)
const perPage = ref(50)
interface SearchRow {
  column: string
  operator: string
  keyword: string
}
interface SelectRow {
  func: string
  column: string
}
const selectRows = ref<SelectRow[]>([{ func: '', column: '' }])
const searchRows = ref<SearchRow[]>([{ column: '', operator: 'LIKE', keyword: '' }])
const sortColumn = ref('')
const sortDesc = ref(false)
const selectedRowIndices = ref<Set<number>>(new Set())
let nextRowId = 1
const editingRows = ref<Record<number, { buffer: Record<string,any>, isNew: boolean }>>({})
const functions = ref<{ value: string; label: string }[]>([])
const wholeResult = ref(false)
const exportMethod = ref('open')
const exportFormat = ref('sql')
const exportFormatOptions = computed(() => [
  { value: 'sql', label: t('tableData.sql') },
  { value: 'csv', label: t('tableData.csv') },
  { value: 'tsv', label: t('tableData.tsv') }
])

const tableScrollRef = ref<HTMLElement | null>(null)
const scrollbarMirrorRef = ref<HTMLElement | null>(null)
const tableWidth = ref(0)

const feedbackItems = ref<SqlFeedbackItem[]>([])

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)
const table = computed(() => route.params.table as string)

const isTableLevel = computed(() => !!connection.value && !!database.value && !!table.value)

const totalPages = computed(() => Math.ceil(total.value / perPage.value) || 1)
const allRowsSelected = computed(() => rows.value.length > 0 && selectedRowIndices.value.size === rows.value.length)
const pageNumbers = computed(() => {
  const tp = totalPages.value
  if (tp <= 10) return Array.from({ length: tp }, (_, i) => i + 1)
  const p = page.value
  const pages: (number | string)[] = [1]
  let start = Math.max(2, p - 2)
  let end = Math.min(tp - 1, p + 2)
  if (start > 2) pages.push('...')
  for (let i = start; i <= end; i++) pages.push(i)
  if (end < tp - 1) pages.push('...')
  if (tp > 1) pages.push(tp)
  return pages
})

onMounted(() => {
  fetchData()
  loadFunctions()
})

async function loadFunctions() {
  const conn = connection.value
  if (!conn) return
  try {
    const result = await api.getFunctions(conn)
    if (result) {
      functions.value = [
        { value: '', label: t('tableData.none') },
        ...result.map(f => ({ value: f.name, label: f.name })),
      ]
    }
  } catch { /* ignore */ }
}

function onTableScroll() {
  const mirror = scrollbarMirrorRef.value
  const scroll = tableScrollRef.value
  if (mirror && scroll) {
    mirror.scrollLeft = scroll.scrollLeft
  }
}

function onMirrorScroll() {
  const scroll = tableScrollRef.value
  const mirror = scrollbarMirrorRef.value
  if (scroll && mirror) {
    scroll.scrollLeft = mirror.scrollLeft
  }
}

function updateTableWidth() {
  const scroll = tableScrollRef.value
  if (scroll) {
    nextTick(() => {
      tableWidth.value = scroll.scrollWidth
    })
  }
}

async function fetchData() {
  const conn = connection.value
  if (!conn || !table.value) return
  loading.value = true
  try {
    const tbl = table.value

    if (allColumns.value.length === 0) {
      try {
        const schemaResult = await api.showColumns(conn, tbl)
        if (schemaResult.data) {
          allColumns.value = schemaResult.data
          if (!sortColumn.value) {
            const pk = allColumns.value.find(c => c.key === 'PRI')
            if (pk) {
              sortColumn.value = pk.name
              sortDesc.value = true
            }
          }
        }
      } catch { /* ignore */ }
    }

    const whereConditions = buildWhereConditions()
    const countResult = await api.countData(conn, tbl, whereConditions)
    total.value = countResult.data ?? 0

    const selectColumns = selectRows.value
      .filter(r => r.column)
      .map(r => ({ name: r.column, func: r.func || undefined }))

    const result = await api.selectData(conn, tbl, {
      columns: selectColumns.length > 0 ? selectColumns : [],
      where_conditions: whereConditions,
      order_by: sortColumn.value ? [{ column: sortColumn.value, desc: sortDesc.value }] : [],
      limit: perPage.value,
      offset: (page.value - 1) * perPage.value,
    })
    if (result.data?.columns) {
      columns.value = result.data.columns.map((c: any) => ({
        name: c.name || '',
        data_type: c.data_type || 'text',
        key: c.key || null,
        comment: c.comment || null,
      }))
    }
    if (result.data?.rows) {
      rows.value = result.data.rows
      for (const row of rows.value) {
        if (row._uid === undefined) row._uid = nextRowId++
      }
    }
    if (columns.value.length === 0 && allColumns.value.length > 0) {
      columns.value = allColumns.value.map(c => ({ name: c.name, data_type: c.data_type, key: c.key, comment: c.comment }))
    }
    const d = new Date()
    const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
    feedbackItems.value = [{
      label: `${connection.value}.${database.value}.${table.value}`,
      success: true,
      sql: result.sql || '',
      duration: result.duration_ms ?? 0,
      time,
    }]
  } catch { /* ignore */ }
  loading.value = false
  await nextTick()
  updateTableWidth()
}

function buildWhereConditions(): { column: string; operator: string; value: string }[] {
  const conds: { column: string; operator: string; value: string }[] = []
  for (const sr of searchRows.value) {
    if (!sr.keyword || !sr.column) continue
    conds.push({ column: sr.column, operator: sr.operator, value: sr.keyword })
  }
  return conds
}

function getTypeIcon(dataType: string): string {
  const t = dataType.toLowerCase()
  if (t.includes('int') || ['decimal', 'float', 'double', 'real'].includes(t)) return 'numeric'
  if (t.includes('char') || t.includes('text')) return 'text'
  if (t.includes('date') || t.includes('time') || t === 'year') return 'calendar'
  if (t.includes('blob') || t.includes('binary')) return 'file'
  if (['boolean', 'bit'].includes(t)) return 'toggle-switch'
  if (t === 'json') return 'code-json'
  if (['enum', 'set'].includes(t)) return 'menu'
  if (t.includes('geometry') || t.includes('point') || t.includes('linestring') || t.includes('polygon')) return 'shape'
  return 'help-circle-outline'
}


function onSearch() {
  page.value = 1
  fetchData()
}

function onSelectColumnChange(i: number) {
  if (i === selectRows.value.length - 1 && selectRows.value[i].column) {
    selectRows.value.push({ func: '', column: '' })
  }
}

function onSearchColumnChange(i: number) {
  if (i === searchRows.value.length - 1 && searchRows.value[i].column) {
    searchRows.value.push({ column: '', operator: 'LIKE', keyword: '' })
  }
}

function toggleRowIndex(i: number) {
  const s = new Set(selectedRowIndices.value)
  if (s.has(i)) s.delete(i); else s.add(i)
  selectedRowIndices.value = s
}

function toggleAllRows() {
  if (allRowsSelected.value) {
    selectedRowIndices.value = new Set()
  } else {
    selectedRowIndices.value = new Set(rows.value.map((_, i) => i))
  }
}

async function deleteRows() {
  const conn = connection.value
  const tbl = table.value
  if (!conn || !tbl) return
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  try {
    if (wholeResult.value) {
      if (!confirm(`Delete all ${total.value} matching row(s)?`)) return
      const where = buildWhereConditions()
      const result = await api.deleteData(conn, tbl, { where_conditions: where })
      feedbackItems.value = [{
        label: `${conn}.${database.value}`,
        success: true,
        sql: result.sql,
        duration: result.duration_ms,
        time,
      }]
    } else {
      const pkCols = columns.value.filter(c => c.key === 'PRI')
      if (pkCols.length === 0) {
        alert(t('tableData.noPrimaryKey'))
        return
      }
      const indices = [...selectedRowIndices.value]
      if (indices.length === 0) return
      if (!confirm(`Delete ${indices.length} row(s)?`)) return
      const where = pkCols.map(pk => {
        const values = indices.map(i => {
          const v = rows.value[i][pk.name]
          return v === null || v === undefined ? 'NULL' : String(v)
        })
        return { column: pk.name, operator: 'IN', value: values.join(',') }
      })
      const result = await api.deleteData(conn, tbl, { where_conditions: where, limit: indices.length })
      feedbackItems.value = [{
        label: `${conn}.${database.value}`,
        success: true,
        sql: result.sql,
        duration: result.duration_ms,
        time,
      }]
    }
  } catch {
    feedbackItems.value = [{
      label: `${conn}.${database.value}`,
      success: false,
      sql: '',
      duration: 0,
      time,
    }]
  }
  selectedRowIndices.value = new Set()
  wholeResult.value = false
  await fetchData()
}

function startEdit(i: number) {
  const row = rows.value[i]
  if (!row) return
  if (row._uid === undefined) row._uid = nextRowId++
  editingRows.value[row._uid] = { buffer: { ...row }, isNew: false }
}

function copyRow(row: any) {
  const copy: Record<string, any> = { _uid: nextRowId++ }
  for (const col of columns.value) {
    copy[col.name] = row[col.name] ?? ''
  }
  rows.value.push(copy)
  editingRows.value[copy._uid] = { buffer: copy, isNew: true }
}

function startAdd() {
  const blank: Record<string, any> = { _uid: nextRowId++ }
  for (const col of columns.value) {
    blank[col.name] = ''
  }
  rows.value.unshift(blank)
  editingRows.value[blank._uid] = { buffer: blank, isNew: true }
}

function cancelEdit(uid: number) {
  const entry = editingRows.value[uid]
  if (!entry) return
  if (entry.isNew) {
    const idx = rows.value.findIndex(r => r._uid === uid)
    if (idx !== -1) rows.value.splice(idx, 1)
  }
  delete editingRows.value[uid]
}

async function saveEdit(uid: number) {
  const entry = editingRows.value[uid]
  if (!entry) return
  const buf = entry.buffer
  const isNew = entry.isNew
  const conn = connection.value
  const tbl = table.value
  if (!conn || !tbl) return
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`

  try {
    if (isNew) {
      const cols: string[] = []
      const vals: string[] = []
      for (const col of columns.value) {
        const v = buf[col.name]
        if (v !== '' && v !== null && v !== undefined) {
          cols.push(col.name)
          vals.push(String(v))
        }
      }
      if (cols.length === 0) { cancelEdit(uid); return }
      await api.insertData(conn, tbl, { columns: cols, values: [vals] })
    } else {
      const idx = rows.value.findIndex(r => r._uid === uid)
      if (idx === -1) return
      const row = rows.value[idx]
      const sets: { column: string; value: string }[] = []
      const pkCols = columns.value.filter(c => c.key === 'PRI')
      const whereCols = pkCols.length > 0 ? pkCols : columns.value
      for (const col of columns.value) {
        const oldVal = row[col.name]
        const newVal = buf[col.name]
        if (String(newVal) !== String(oldVal)) {
          sets.push({ column: col.name, value: String(newVal) })
        }
      }
      const where: { column: string; operator: string; value: string }[] = []
      for (const col of whereCols) {
        const oldVal = row[col.name]
        if (oldVal === null || oldVal === undefined) {
          where.push({ column: col.name, operator: '=', value: 'NULL' })
        } else {
          where.push({ column: col.name, operator: '=', value: String(oldVal) })
        }
      }
      if (sets.length === 0) { cancelEdit(uid); return }
      await api.updateData(conn, tbl, { sets, where_conditions: where, limit: 1 })
    }
    feedbackItems.value = [{
      label: `${conn}.${database.value}`,
      success: true,
      sql: '',
      duration: 0,
      time,
    }]
  } catch {
    feedbackItems.value = [{
      label: `${conn}.${database.value}`,
      success: false,
      sql: '',
      duration: 0,
      time,
    }]
  }
  delete editingRows.value[uid]
  selectedRowIndices.value = new Set()
  await fetchData()
}

async function onExport() {
  const conn = connection.value
  const tbl = table.value
  if (!conn || !tbl) return

  const fmt = exportFormat.value

  if (wholeResult.value) {
    const where = buildWhereConditions()
    const cols = columns.value.map(c => c.name)
    const body = {
      method: exportMethod.value,
      format: fmt,
      columns: cols,
      where_conditions: where,
      order_by: sortColumn.value ? [{ column: sortColumn.value, desc: sortDesc.value }] : [],
      database: database.value,
    }
    try {
      const res = await fetch(`/api/connections/${encodeURIComponent(conn)}/tables/${encodeURIComponent(tbl)}/export`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(body),
      })
      if (!res.ok) {
        const err = await res.json().catch(() => ({}))
        alert(err.error || `HTTP ${res.status}`)
        return
      }
      const blob = await res.blob()
      const url = URL.createObjectURL(blob)
      if (exportMethod.value === 'save') {
        const disposition = res.headers.get('Content-Disposition')
        let filename = `${tbl}.${fmt}`
        if (disposition) {
          const match = disposition.match(/filename="(.+)"/)
          if (match) filename = match[1]
        }
        const a = document.createElement('a')
        a.href = url
        a.download = filename
        a.click()
      } else {
        window.open(url)
      }
      URL.revokeObjectURL(url)
    } catch (e: any) {
      alert(e.message || 'Export failed')
    }
    wholeResult.value = false
    return
  }

  const selected = Array.from(selectedRowIndices.value).map(i => rows.value[i]).filter(Boolean)
  const data = selected.length > 0 ? selected : rows.value
  if (data.length === 0) return
  const cols = columns.value.map(c => c.name)
  let content = ''
  if (fmt === 'csv') {
    content = cols.join(',') + '\n' + data.map(r => cols.map(c => {
      const v = r[c]
      if (v === null || v === undefined) return ''
      return `"${String(v).replace(/"/g, '""')}"`
    }).join(',')).join('\n')
  } else if (fmt === 'tsv') {
    content = cols.join('\t') + '\n' + data.map(r => cols.map(c => {
      const v = r[c]
      if (v === null || v === undefined) return ''
      return String(v).replace(/\t/g, ' ')
    }).join('\t')).join('\n')
  } else if (fmt === 'sql') {
    const esc = (v: any) => v === null || v === undefined ? 'NULL' : `'${String(v).replace(/'/g, "\\'")}'`
    content = data.map(r => {
      const vals = cols.map(c => esc(r[c]))
      return `INSERT INTO \`${tbl}\` (\`${cols.join('`, `')}\`) VALUES (${vals.join(', ')})` + ';'
    }).join('\n') + '\n'
  }
  const blob = new Blob([content], { type: 'text/plain;charset=utf-8' })
  const url = URL.createObjectURL(blob)
  if (exportMethod.value === 'save') {
    const a = document.createElement('a')
    a.href = url
    a.download = `${tbl}.${fmt}`
    a.click()
  } else {
    window.open(url)
  }
  URL.revokeObjectURL(url)
}

function onPageChange(p: number) {
  page.value = p
  fetchData()
}

function goStructure() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(table.value)}/structure`)
}

function goAlterTable() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(table.value)}/alter`)
}

function goInsert() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(table.value)}/insert`)
}

function goSql() {
  router.push(`/query/${encodeURIComponent(connection.value)}`)
}

function pad(n: number): string {
  return n.toString().padStart(2, '0')
}




watch(
  () => [route.params.connection, route.params.db, route.params.table],
  ([, , newTable], [, , oldTable]) => {
    if (newTable !== oldTable) {
      selectRows.value = [{ func: '', column: '' }]
      searchRows.value = [{ column: '', operator: 'LIKE', keyword: '' }]
      sortColumn.value = ''
      sortDesc.value = false
      page.value = 1
      selectedRowIndices.value = new Set()
      editingRows.value = {}
      wholeResult.value = false
    }
    fetchData()
    loadFunctions()
  },
)
</script>

<template>
  <div>
    <div class="page-header">
      <template v-if="isTableLevel">{{ $t('tableData.select') }}: {{ table }}</template>
      <template v-else>{{ connection }}</template>
    </div>

    <div class="page-content">
      <SqlFeedback :items="feedbackItems" :connection="connection" @remove="(i: number) => feedbackItems.splice(i, 1)" />

      <!-- Table Level -->
      <template v-if="isTableLevel">
        <div class="top-actions">
          <a href="#" class="active">{{ $t('tableData.selectData') }}</a>
          <a href="#" @click.prevent="goStructure">{{ $t('tableData.showStructure') }}</a>
          <a href="#" @click.prevent="goAlterTable">{{ $t('tableData.alterTable') }}</a>
          <a href="#" @click.prevent="goInsert">{{ $t('tableData.newItem') }}</a>
        </div>

        <fieldset style="display:inline-block;margin-bottom:8px">
          <legend>{{ $t('tableData.select') }}</legend>
          <div class="fieldset-content" style="flex-direction:column;align-items:stretch">
            <div v-for="(row, i) in selectRows" :key="i" style="display:flex;gap:8px;align-items:center">
              <SearchableSelect v-model="row.func" :options="functions" size="small" style="width:100px" />
              <SearchableSelect v-model="row.column" :options="[{value:'',label:$t('tableData.selectColumn')},...allColumns.map(c=>({value:c.name,label:c.name}))]" size="small" style="width:120px" @change="onSelectColumnChange(i)" />
            </div>
          </div>
        </fieldset>

        <fieldset style="display:inline-block;margin-bottom:8px;margin-left:8px">
          <legend>{{ $t('tableData.search') }}</legend>
          <div class="fieldset-content" style="flex-direction:column;align-items:stretch">
            <div v-for="(sr, i) in searchRows" :key="i" style="display:flex;gap:8px;align-items:center">
              <SearchableSelect v-model="sr.column" :options="[{value:'',label:$t('tableData.selectColumn')},...allColumns.map(c=>({value:c.name,label:c.name}))]" size="small" style="width:120px" @change="onSearchColumnChange(i)" />
              <SearchableSelect v-model="sr.operator" :options="[{value:'=',label:'='},{value:'>',label:'>'},{value:'<',label:'<'},{value:'>=',label:'>='},{value:'<=',label:'<='},{value:'!=',label:'!='},{value:'LIKE',label:'LIKE'},{value:'NOT LIKE',label:'NOT LIKE'}]" size="small" style="width:100px" />
              <input v-model="sr.keyword" type="text" style="width:150px" :placeholder="$t('tableData.value')" @keyup.enter="onSearch">
            </div>
          </div>
        </fieldset>

        <fieldset style="display:inline-block;margin-bottom:8px;margin-left:8px">
          <legend>{{ $t('tableData.sort') }}</legend>
          <div class="fieldset-content">
            <SearchableSelect v-model="sortColumn" :options="[{value:'',label:$t('tableData.selectColumn')},...allColumns.map(c=>({value:c.name,label:c.name}))]" size="small" style="width:120px" />
            <label style="font-size:12px;white-space:nowrap">
              <input type="checkbox" v-model="sortDesc"> DESC
            </label>
          </div>
        </fieldset>

        <fieldset style="display:inline-block;margin-bottom:8px;margin-left:8px">
          <legend>{{ $t('tableData.limit') }}</legend>
          <div class="fieldset-content">
            <input v-model.number="perPage" type="number" style="width:60px">
          </div>
        </fieldset>

        <fieldset style="display:inline-block;margin-bottom:8px;margin-left:8px">
          <legend>{{ $t('tableData.action') }}</legend>
          <div class="fieldset-content">
            <button @click="onSearch">{{ $t('tableData.select') }}</button>
          </div>
        </fieldset>

        <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

        <div v-else class="table-container">
          <div class="table-scroll" ref="tableScrollRef" @scroll="onTableScroll">
            <table>
              <thead>
                  <tr>
                    <th style="width:70px"><a href="#" @click.prevent="startAdd" :title="$t('tableData.addRow')"><i class="mdi mdi-plus" /></a></th>
                    <th style="width:30px"><input type="checkbox" :checked="allRowsSelected" @change="toggleAllRows"></th>
                    <th v-for="col in columns" :key="col.name" :title="col.comment || col.name">
                      <i class="mdi" :class="'mdi-' + getTypeIcon(col.data_type)" /> {{ col.name }}
                    </th>
                  </tr>
                </thead>
                <tbody>
                  <tr v-for="(row, ri) in rows" :key="row._uid ?? ri" :class="{ selected: selectedRowIndices.has(ri) }">
                    <td style="white-space:nowrap">
                      <template v-if="row._uid !== undefined && editingRows[row._uid]">
                        <a href="#" @click.prevent="saveEdit(row._uid)" :title="$t('tableData.save')"><i class="mdi mdi-check" /></a>
                        <a href="#" @click.prevent="cancelEdit(row._uid)" :title="$t('tableData.cancel')" style="margin-left:4px"><i class="mdi mdi-close" /></a>
                      </template>
                      <template v-else>
                        <a href="#" @click.prevent="startEdit(ri)" :title="$t('tableData.editRow')"><i class="mdi mdi-pencil" /></a>
                        <a href="#" @click.prevent="copyRow(row)" :title="$t('tableData.copyRow')" style="margin-left:4px"><i class="mdi mdi-content-copy" /></a>
                      </template>
                    </td>
                    <td><input type="checkbox" :checked="selectedRowIndices.has(ri)" @change="toggleRowIndex(ri)"></td>
                    <td v-for="col in columns" :key="col.name">
                      <template v-if="row._uid !== undefined && editingRows[row._uid]">
                        <input v-model="editingRows[row._uid].buffer[col.name]" type="text" class="inline-edit-input">
                      </template>
                      <template v-else>
                        {{ row[col.name] !== null && row[col.name] !== undefined ? row[col.name] : '' }}<span v-if="row[col.name] === null || row[col.name] === undefined" class="null-value">{{ $t('tableData.null') }}</span>
                      </template>
                    </td>
                  </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div class="float-bar">
          <div class="scrollbar-mirror" ref="scrollbarMirrorRef" @scroll="onMirrorScroll">
            <div :style="{ width: tableWidth + 'px', height: '1px' }"></div>
          </div>
          <div class="float-bar-content">
            <fieldset>
              <legend>{{ $t('tableData.page') }}</legend>
              <div class="fieldset-content">
                <a
                  v-for="(p, i) in pageNumbers"
                  :key="i"
                  href="#"
                  class="page-link"
                  :class="{ current: p === page }"
                  @click.prevent="typeof p === 'number' && onPageChange(p)"
                >{{ p }}</a>
              </div>
            </fieldset>
            <fieldset>
              <legend>{{ $t('tableData.wholeResult') }}</legend>
              <div class="fieldset-content">
                <label style="display:flex;align-items:center;gap:4px">
                  <input type="checkbox" v-model="wholeResult">
                  <span>{{ $t('tableData.wholeResult') }}</span>
                </label>
                <span>{{ total }} {{ $t('tableData.rows') }}</span>
              </div>
            </fieldset>
            <fieldset>
              <legend>{{ $t('tableData.selected') }}</legend>
              <div class="fieldset-content">
                <span style="font-size:12px">{{ selectedRowIndices.size }} {{ $t('tableData.rows') }}</span>
                <button :disabled="selectedRowIndices.size === 0" @click="deleteRows">{{ $t('tableData.deleteSelected') }}</button>
              </div>
            </fieldset>
            <fieldset>
              <legend>{{ $t('tableData.exportFormat') }}</legend>
              <div class="fieldset-content">
                <SearchableSelect v-model="exportMethod" :options="[{value:'open',label:$t('tableData.open')},{value:'save',label:$t('tableData.save')}]" size="small" style="width:80px" />
                <SearchableSelect v-model="exportFormat" :options="exportFormatOptions" size="small" style="width:80px" />
                <button @click="onExport">{{ $t('tableData.export') }}</button>
              </div>
            </fieldset>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<style scoped>
.table-container {
  display: flex;
  flex-direction: column;
}

.table-scroll {
  overflow-x: auto;
}

.table-scroll::-webkit-scrollbar {
  display: none;
}

.float-bar {
  position: sticky;
  bottom: 0;
  background: var(--adminer-bg, #fff);
  border-top: 2px solid var(--adminer-border, #ccc);
  z-index: 10;
}

.float-bar-content {
  display: flex;
  gap: 12px;
  align-items: center;
  flex-wrap: wrap;
  padding: 6px 10px;
}

.scrollbar-mirror {
  overflow-x: auto;
  overflow-y: hidden;
  height: 10px;
}

.scrollbar-mirror::-webkit-scrollbar {
  height: 10px;
}

.scrollbar-mirror::-webkit-scrollbar-track {
  background: transparent;
}

.scrollbar-mirror::-webkit-scrollbar-thumb {
  background: #aaa;
  border-radius: 5px;
}

.float-bar fieldset {
  margin: 0;
  padding: 4px 8px;
}

.float-bar .fieldset-content {
  display: flex;
  gap: 6px;
  align-items: center;
  font-size: 12px;
}

.float-bar button:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.inline-edit-input {
  width: 100%;
  min-width: 80px;
  font-size: 13px;
  padding: 1px 4px;
  border: 1px solid #999;
  font-family: var(--adminer-font);
  box-sizing: border-box;
}

.page-link {
  padding: 1px 6px;
  border: 1px solid #999;
  font-size: 12px;
  text-decoration: none;
  color: inherit;
  cursor: pointer;
}

.page-link.current {
  color: #000;
  border: none;
  cursor: default;
}

.page-link:not(.current):hover {
  background: #eee;
}

.null-value {
  color: #999;
  font-style: italic;
  opacity: 0.7;
}
</style>
