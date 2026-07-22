<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { api } from '../api'
import SqlFeedback from '../components/SqlFeedback.vue'
import type { SqlFeedbackItem } from '../components/SqlFeedback.vue'

const { t } = useI18n()

interface TableDetail {
  table_name: string
  engine: string | null
  table_collation: string | null
  table_comment: string | null
  auto_increment: number | null
  table_rows: number | null
  data_length: number | null
  index_length: number | null
  data_free: number | null
}

const route = useRoute()
const router = useRouter()

const tablesDetail = ref<TableDetail[]>([])
const loadingTables = ref(false)
const selectedTables = ref<Set<string>>(new Set())
const feedbackItems = ref<SqlFeedbackItem[]>([])

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)

onMounted(() => {
  fetchTableDetails()
})

async function fetchTableDetails() {
  const conn = connection.value
  const db = database.value
  if (!conn || !db) return
  loadingTables.value = true
  tablesDetail.value = []
  try {
    const result = await api.listTables(conn, db)
    tablesDetail.value = result.data as unknown as TableDetail[]
  } catch { /* ignore */ }
  loadingTables.value = false
}

function toggleSelectAll() {
  if (selectedTables.value.size === tablesDetail.value.length) {
    selectedTables.value = new Set()
  } else {
    selectedTables.value = new Set(tablesDetail.value.map(t => t.table_name))
  }
}

function toggleTable(name: string) {
  const s = new Set(selectedTables.value)
  if (s.has(name)) s.delete(name); else s.add(name)
  selectedTables.value = s
}

function goAlterDatabase() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/alter`)
}

function goPrivileges() {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/privileges`)
}

function goTable(tbl: string) {
  router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(tbl)}`)
}

function formatBytes(bytes: number | null): string {
  if (bytes === null || bytes === undefined) return ''
  if (bytes <= 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB', 'TB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i]
}

function pad(n: number): string {
  return n.toString().padStart(2, '0')
}

async function dropTables() {
  if (!confirm(`DROP tables: ${Array.from(selectedTables.value).join(', ')}?`)) return
  const conn = connection.value
  const db = database.value
  const results = await api.dropTables(conn, db, Array.from(selectedTables.value))
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  feedbackItems.value = results.map(r => ({
    label: `${conn}.${db}`,
    success: r.success,
    sql: r.data?.sql ?? `DROP TABLE`,
    duration: r.data?.duration_ms ?? 0,
    time,
  }))
  selectedTables.value = new Set()
  await fetchTableDetails()
}

async function truncateTables() {
  if (!confirm(`TRUNCATE tables: ${Array.from(selectedTables.value).join(', ')}?`)) return
  const conn = connection.value
  const db = database.value
  const results = await api.truncateTables(conn, db, Array.from(selectedTables.value))
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  feedbackItems.value = results.map(r => ({
    label: `${conn}.${db}`,
    success: r.success,
    sql: r.data?.sql ?? `TRUNCATE TABLE`,
    duration: r.data?.duration_ms ?? 0,
    time,
  }))
  selectedTables.value = new Set()
  await fetchTableDetails()
}

async function repairTables() {
  const conn = connection.value
  const db = database.value
  const results = await api.repairTables(conn, db, Array.from(selectedTables.value))
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  feedbackItems.value = results.map(r => ({
    label: `${conn}.${db}`,
    success: r.success,
    sql: r.data?.sql ?? `REPAIR TABLE`,
    duration: r.data?.duration_ms ?? 0,
    time,
  }))
}

async function optimizeTables() {
  const conn = connection.value
  const db = database.value
  const results = await api.optimizeTables(conn, db, Array.from(selectedTables.value))
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  feedbackItems.value = results.map(r => ({
    label: `${conn}.${db}`,
    success: r.success,
    sql: r.data?.sql ?? `OPTIMIZE TABLE`,
    duration: r.data?.duration_ms ?? 0,
    time,
  }))
}

async function analyzeTables() {
  const conn = connection.value
  const db = database.value
  const results = await api.analyzeTables(conn, db, Array.from(selectedTables.value))
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  feedbackItems.value = results.map(r => ({
    label: `${conn}.${db}`,
    success: r.success,
    sql: r.data?.sql ?? `ANALYZE TABLE`,
    duration: r.data?.duration_ms ?? 0,
    time,
  }))
}

async function checkTables() {
  const conn = connection.value
  const db = database.value
  const results = await api.checkTables(conn, db, Array.from(selectedTables.value))
  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
  feedbackItems.value = results.map(r => ({
    label: `${conn}.${db}`,
    success: r.success,
    sql: r.data?.sql ?? `CHECK TABLE`,
    duration: r.data?.duration_ms ?? 0,
    time,
  }))
}

watch(
  () => [route.params.connection, route.params.db],
  () => {
    fetchTableDetails()
  },
)
</script>

<template>
  <div>
    <div class="page-header">Database: {{ database }}</div>

    <div class="page-content">
      <SqlFeedback :items="feedbackItems" :connection="connection" @remove="(i: number) => feedbackItems.splice(i, 1)" />

      <div class="top-actions">
        <a href="#" @click.prevent="goAlterDatabase">{{ $t('tableList.alterDatabase') }}</a>
        <a href="#" @click.prevent="goPrivileges">{{ $t('tableList.privileges') }}</a>
      </div>

      <h3 style="font-size:14px;font-weight:normal;margin-bottom:8px">{{ $t('tableList.title') }}</h3>

      <div v-if="loadingTables" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <table v-else-if="tablesDetail.length">
        <thead>
          <tr>
            <th style="width:30px"><input type="checkbox" :checked="selectedTables.size === tablesDetail.length && tablesDetail.length > 0" @change="toggleSelectAll"></th>
            <th>{{ $t('tableList.table') }}</th>
            <th>{{ $t('tableList.engine') }}</th>
            <th>{{ $t('tableList.collation') }}</th>
            <th class="num">{{ $t('tableList.dataLength') }}</th>
            <th class="num">{{ $t('tableList.indexLength') }}</th>
            <th class="num">{{ $t('tableList.dataFree') }}</th>
            <th class="num">{{ $t('tableList.autoIncrement') }}</th>
            <th class="num">{{ $t('tableList.rows') }}</th>
            <th>{{ $t('tableList.comment') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="t in tablesDetail" :key="t.table_name" :class="{ selected: selectedTables.has(t.table_name) }">
            <td><input type="checkbox" :checked="selectedTables.has(t.table_name)" @change="toggleTable(t.table_name)"></td>
            <td><a href="#" @click.prevent="goTable(t.table_name)">{{ t.table_name }}</a></td>
            <td>{{ t.engine }}</td>
            <td>{{ t.table_collation }}</td>
            <td class="num" :title="String(t.data_length)">{{ formatBytes(t.data_length) }}</td>
            <td class="num" :title="String(t.index_length)">{{ formatBytes(t.index_length) }}</td>
            <td class="num" :title="String(t.data_free)">{{formatBytes(t.data_free) }}</td>
            <td class="num">{{ t.auto_increment ?? '' }}</td>
            <td class="num">{{ t.table_rows ?? '' }}</td>
            <td>{{ t.table_comment }}</td>
          </tr>
        </tbody>
      </table>

      <div v-if="selectedTables.size > 0" class="bottom-bar">
        <span style="font-size:12px">Selected ({{ selectedTables.size }})</span>
        <button @click="analyzeTables">{{ $t('tableList.analyze') }}</button>
        <button @click="optimizeTables">{{ $t('tableList.optimize') }}</button>
        <button @click="checkTables">{{ $t('tableList.check') }}</button>
        <button @click="repairTables">{{ $t('tableList.repair') }}</button>
        <button @click="truncateTables">{{ $t('tableList.truncate') }}</button>
        <button class="danger" @click="dropTables">{{ $t('tableList.drop') }}</button>
      </div>
    </div>
  </div>
</template>
