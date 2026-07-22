<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { api } from '../api'

import type { Connection } from '../types'

const { t } = useI18n()

const route = useRoute()
const router = useRouter()

const connections = ref<Connection[]>([])
const activeConn = ref('')
const databases = ref<{ name: string; collation: string; tables: number; size: number }[]>([])
const selectedDbs = ref<Set<string>>(new Set())
const loading = ref(true)
const computing = ref(false)
const message = ref('')
const mysqlVersion = ref('')

onMounted(async () => {
  try {
    connections.value = await api.listConnections()
    const connParam = route.params.connection as string
    if (connParam) {
      activeConn.value = connParam
      await loadDatabases()
      await loadVersion()
    }
  } catch { /* ignore */ }
  loading.value = false
})

async function loadVersion() {
  if (!activeConn.value) return
  try {
    const result = await api.version(activeConn.value)
    mysqlVersion.value = result.data ?? ''
  } catch { /* ignore */ }
}

async function loadDatabases() {
  if (!activeConn.value) return
  try {
    const result = await api.listDatabases(activeConn.value)
    databases.value = (result.data || []).map((r: any) => ({
      name: r.name || '',
      collation: r.collation || '',
      tables: 0,
      size: 0,
    }))
  } catch { /* ignore */ }
}

async function computeSizes() {
  if (!activeConn.value) return
  computing.value = true
  for (const db of databases.value) {
    try {
      const [countResult, sizeResult] = await Promise.all([
        api.tableCount(activeConn.value, db.name),
        api.tableSizes(activeConn.value, db.name),
      ])
      if (countResult.data != null) db.tables = Number(countResult.data) || 0
      if (sizeResult.data != null) db.size = Number(sizeResult.data) || 0
    } catch { /* ignore */ }
  }
  computing.value = false
}

function toggleSelectAll() {
  if (selectedDbs.value.size === databases.value.length) {
    selectedDbs.value = new Set()
  } else {
    selectedDbs.value = new Set(databases.value.map(d => d.name))
  }
}

function toggleDb(name: string) {
  const s = new Set(selectedDbs.value)
  if (s.has(name)) s.delete(name)
  else s.add(name)
  selectedDbs.value = s
}

function selectDb(name: string) {
  router.push(`/browse/${encodeURIComponent(activeConn.value)}/${encodeURIComponent(name)}`)
}

async function dropDatabases() {
  if (selectedDbs.value.size === 0) return
  const names = Array.from(selectedDbs.value).join(', ')
  if (!confirm(`DROP databases: ${names}?`)) return
  for (const name of selectedDbs.value) {
    try {
      await api.dropDatabase(activeConn.value, name)
    } catch { /* ignore */ }
  }
  selectedDbs.value = new Set()
  await loadDatabases()
}

function formatSize(bytes: number): string {
  if (bytes <= 0 || !isFinite(bytes)) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return (bytes / Math.pow(1024, i)).toFixed(1) + ' ' + units[i]
}

function goCreateDatabase() {
  router.push(`/create-database/${encodeURIComponent(activeConn.value)}`)
}

function goPrivileges() {
  router.push(`/privileges/${encodeURIComponent(activeConn.value)}`)
}

function goProcessList() {
  router.push(`/process-list/${encodeURIComponent(activeConn.value)}`)
}

function goVariables() {
  router.push(`/variables/${encodeURIComponent(activeConn.value)}`)
}

function goStatus() {
  router.push(`/status/${encodeURIComponent(activeConn.value)}`)
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('databaseList.title') }}</div>
    <div class="page-content">
      <div class="top-actions">
        <a href="#" @click.prevent="goCreateDatabase">{{ $t('databaseList.newDatabase') }}</a>
        <a href="#" @click.prevent="goPrivileges">{{ $t('databaseList.privileges') }}</a>
        <a href="#" @click.prevent="goProcessList">{{ $t('databaseList.processList') }}</a>
        <a href="#" @click.prevent="goVariables">{{ $t('databaseList.variables') }}</a>
        <a href="#" @click.prevent="goStatus">{{ $t('databaseList.status') }}</a>
      </div>

      <div v-if="connections.length > 0" style="margin-bottom:8px;font-size:13px">
        MySQL version: <strong>{{ mysqlVersion }}</strong>
        <br>Logged as: <strong>{{ activeConn }}</strong>
      </div>

      <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <template v-else>
        <table>
          <thead>
            <tr>
              <th style="width:30px"><input type="checkbox" :checked="selectedDbs.size === databases.length && databases.length > 0" @change="toggleSelectAll"></th>
              <th>{{ $t('databaseList.name') }} - <a href="#" @click.prevent="loadDatabases">{{ $t('databaseList.refresh') }}</a></th>
              <th>{{ $t('databaseList.collation') }}</th>
              <th style="width:60px">{{ $t('databaseList.tables') }}</th>
              <th style="width:100px">{{ $t('databaseList.size') }} - <a href="#" @click.prevent="computeSizes">{{ $t('databaseList.compute') }}</a></th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="db in databases" :key="db.name" :class="{ selected: selectedDbs.has(db.name) }">
              <td><input type="checkbox" :checked="selectedDbs.has(db.name)" @change="toggleDb(db.name)"></td>
              <td><a href="#" @click.prevent="selectDb(db.name)">{{ db.name }}</a></td>
              <td>{{ db.collation || '?' }}</td>
              <td class="num">{{ db.tables != null ? db.tables : '?' }}</td>
              <td class="num">{{ db.size != null ? formatSize(db.size) : '?' }}</td>
            </tr>
          </tbody>
        </table>
      </template>

      <div class="bottom-bar">
        <span style="font-size:12px">Selected ({{ selectedDbs.size }})</span>
        <button class="danger" @click="dropDatabases">{{ $t('databaseList.drop') }}</button>
      </div>
    </div>
  </div>
</template>
