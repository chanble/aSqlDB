<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import QueryResultTable from '../components/QueryResultTable.vue'
import SelectResultDisplay from '../components/SelectResultDisplay.vue'
import SqlFeedback from '../components/SqlFeedback.vue'
import type { SqlFeedbackItem } from '../components/SqlFeedback.vue'
import { api } from '../api'
import type { Connection } from '../types'
import { Codemirror } from 'vue-codemirror'
import { sql as sqlLang } from '@codemirror/lang-sql'
import { autocompletion } from '@codemirror/autocomplete'
import { EditorView, keymap } from '@codemirror/view'
import { indentWithTab } from '@codemirror/commands'
import { useQueryHistory } from '../composables/useQueryHistory'

const { t } = useI18n()

const route = useRoute()

const { history, addEntry: addHistoryEntry, clearHistory } = useQueryHistory()

const sql = ref('SELECT * FROM `table` LIMIT 50')

type QueryPageResult =
  | { type: 'select'; success: true; rows: Record<string, any>[]; sql?: string }
  | { type: 'table'; success: true; columns: string[]; rows: Record<string, any>[]; duration?: number; sql?: string }
  | { type: 'table'; success: false; columns: string[]; rows: Record<string, any>[]; error: string; sql?: string }

const results = ref<QueryPageResult[]>([])
const loading = ref(false)
const error = ref('')
const connections = ref<Connection[]>([])
const activeConn = ref('')
const limitRows = ref('')
const stopOnError = ref(false)
const showOnlyErrors = ref(false)
const feedbackItems = ref<SqlFeedbackItem[]>([])

const savedSql = localStorage.getItem('sql-query-sql')
if (savedSql) sql.value = savedSql

// --- WebSocket 补全 ---
let ws: WebSocket | null = null
let seq = 0
let wsConnected = false
let reconnectTimer: ReturnType<typeof setTimeout> | null = null
let latestItems: { from: number; options: any[] } | null = null
let debounceTimer: ReturnType<typeof setTimeout> | null = null

function connectWS() {
  ws?.close()
  wsConnected = false
  if (reconnectTimer) clearTimeout(reconnectTimer)
  if (!activeConn.value) return
  const proto = location.protocol === 'https:' ? 'wss:' : 'ws:'
  ws = new WebSocket(`${proto}//${location.host}/api/connections/${encodeURIComponent(activeConn.value)}/complete`)
  ws.onopen = () => { wsConnected = true }
  ws.onclose = () => {
    wsConnected = false
    ws = null
    if (activeConn.value) {
      reconnectTimer = setTimeout(connectWS, 3000)
    }
  }
  ws.onmessage = (e) => {
    const data = JSON.parse(e.data)
    if (data.seq === seq) {
      latestItems = {
        from: data.cursor,
        options: data.items.map((i: any) => ({
          label: i.text,
          type: i.kind.toLowerCase(),
        })),
      }
    }
  }
}

watch(activeConn, connectWS)

const completionExt = autocompletion({
  activateOnTyping: true,
  override: [(ctx) => {
    if (!latestItems) return null

    let from = ctx.pos
    const word = ctx.matchBefore(/\w+/)
    if (word) from = word.from

    const r = { from, options: latestItems.options, filter: false }
    latestItems = null
    return r
  }],
})

const updateListener = EditorView.updateListener.of((update) => {
  if (update.docChanged) {
    if (debounceTimer) clearTimeout(debounceTimer)
    debounceTimer = setTimeout(() => {
      seq++
      if (ws && wsConnected) {
        ws.send(JSON.stringify({
          sql: update.state.doc.toString(),
          cursor: update.state.selection.main.head,
          seq,
        }))
      }
    }, 30)
  }
})

const editorKeymap = keymap.of([
  indentWithTab,
  { key: 'Ctrl-Enter', run: () => { execute(); return true } },
  { key: 'Cmd-Enter', run: () => { execute(); return true } },
])

const extensions = [
  sqlLang(),
  completionExt,
  updateListener,
  editorKeymap,
  EditorView.lineWrapping,
]

// --- 原有逻辑 ---

onMounted(async () => {
  try {
    connections.value = await api.listConnections()
    const connParam = route.params.connection as string
    if (connParam) {
      activeConn.value = connParam
    } else if (connections.value.length > 0) {
      activeConn.value = connections.value[0].name
    }
    const querySql = route.query.sql as string
    if (querySql) {
      sql.value = querySql
    }
  } catch { /* ignore */ }
})

onUnmounted(() => {
  ws?.close()
  if (reconnectTimer) clearTimeout(reconnectTimer)
})

async function execute() {
  if (!activeConn.value) {
    error.value = 'No connection selected'
    return
  }
  if (!sql.value.trim()) return

  localStorage.setItem('asql-query-sql', sql.value)

  loading.value = true
  error.value = ''
  results.value = []
  feedbackItems.value = []

  const startTime = Date.now()

  try {
    const raw = await api.executeQuery(activeConn.value, sql.value)
    const elapsed = Date.now() - startTime

    results.value = raw.map((r: any) => {
      if (r.success) {
        const data = r.data
        let columns: string[] = []
        let rows: Record<string, any>[] = []
        let duration: number | undefined

        if (data?.Select?.data?.rows) {
          return { type: 'select', success: true, rows: data.Select.data.rows }
        }

        let type: 'table' = 'table'

        if (data?.Modify?.data) {
          rows = []
          columns = ['Rows Affected', 'Last Insert ID']
          duration = data.Modify.duration_ms
          rows.push({
            'Rows Affected': data.Modify.data.rows_affected,
            'Last Insert ID': data.Modify.data.last_insert_id ?? 'N/A',
          })
        } else if (data?.Schema?.data) {
          rows = []
          columns = ['Message']
          duration = data.Schema.duration_ms
          rows.push({
            'Message': data.Schema.data.message || '',
          })
        } else if (Array.isArray(data)) {
          rows = data
          if (data.length > 0) {
            columns = Object.keys(data[0])
          }
        }
        return { type, success: true, columns, rows, duration }
      }
      return { type: 'table', success: false, columns: [], rows: [], error: r.error || 'Query failed' }
    })

    const now = new Date()
    const pad = (n: number) => n.toString().padStart(2, '0')
    const time = `${now.getFullYear()}-${pad(now.getMonth() + 1)}-${pad(now.getDate())} ${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`

    const allSuccess = results.value.every(r => r.success)
    feedbackItems.value.push({
      label: activeConn.value,
      success: allSuccess,
      sql: sql.value,
      duration: elapsed,
      time,
    })

    addHistoryEntry({
      time,
      sql: sql.value,
      duration: elapsed + 'ms',
      success: allSuccess,
    })

  } catch (e: any) {
    error.value = e.message || String(e)
  }
  loading.value = false
}

function fromHistory(h: { sql: string }) {
  sql.value = h.sql
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('query.title') }}</div>
    <div class="page-content">
      <!-- Error -->
      <article v-if="error" class="message error" style="margin-bottom:12px">
        {{ error }}
      </article>

      <!-- Results (above input) -->
      <div v-if="results.length > 0" style="margin-bottom:12px">
        <SqlFeedback
          :items="feedbackItems"
          :connection="activeConn"
          @remove="(i: number) => { feedbackItems.splice(i, 1); results.splice(i, 1) }"
        />
        <div v-for="(r, i) in results" :key="i">
          <SelectResultDisplay
            v-if="r.type === 'select'"
            :rows="r.rows"
            :title="results.length > 1 ? `Result ${i + 1}` : undefined"
          />
          <QueryResultTable
            v-else
            :columns="r.columns"
            :rows="r.rows"
            :duration="'duration' in r ? r.duration : undefined"
            :success="r.success"
            :error="'error' in r ? r.error : undefined"
            :title="results.length > 1 ? `Result ${i + 1}` : undefined"
          />
        </div>
      </div>

      <Codemirror
        v-model="sql"
        :extensions="extensions"
        :style="{ minHeight: '150px', border: '1px solid #ccc', borderRadius: '4px', overflow: 'hidden' }"
        :placeholder="$t('query.placeholder')"
      />

      <div style="margin-top:8px;display:flex;align-items:center;gap:8px;flex-wrap:wrap">
        <button @click="execute" :disabled="loading">{{ $t('query.run') }}</button>
        <label style="font-size:12px">
          Limit rows:
          <input v-model="limitRows" type="text" style="width:60px;font-size:12px;padding:1px 4px;border:1px solid #999">
        </label>
        <label style="font-size:12px">
          <input v-model="stopOnError" type="checkbox">
          Stop on error
        </label>
        <label style="font-size:12px">
          <input v-model="showOnlyErrors" type="checkbox">
          Show only errors
        </label>
      </div>

      <!-- History -->
      <div v-if="history.length > 0" class="sql-history" style="margin-top:16px">
        <div class="sql-history-title">
          {{ $t('query.history') }}
          <button @click="clearHistory" style="float:right;font-size:11px;padding:1px 6px">{{ $t('query.clear') }}</button>
        </div>
        <div
          v-for="(h, i) in history"
          :key="i"
          class="sql-history-item"
          @click="fromHistory(h)"
        >
          <span style="color:#666">{{ h.time }}</span>
          <span :style="{ color: h.success ? '#007F00' : 'red' }">
            {{ h.sql.length > 100 ? h.sql.slice(0, 100) + '...' : h.sql }}
          </span>
          <span style="color:#999">({{ h.duration }})</span>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sql-history-item span {
  margin-right: 5px;
}
</style>
