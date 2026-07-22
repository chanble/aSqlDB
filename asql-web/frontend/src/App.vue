<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, provide } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import Sidebar from './components/Sidebar.vue'
import SearchableSelect from './components/SearchableSelect.vue'
import { api } from './api'
import { useHeartbeat } from './composables/useHeartbeat'
import type { Connection } from './types'

const route = useRoute()
const router = useRouter()
const { locale, t } = useI18n()

const connName = computed(() => route.params.connection as string | undefined)
useHeartbeat(connName)

const connections = ref<Connection[]>([])

const dbListRefreshKey = ref(0)
provide('dbListRefreshKey', dbListRefreshKey)

const tableListRefreshKey = ref(0)
provide('tableListRefreshKey', tableListRefreshKey)

const SIDEBAR_MIN = 150
const SIDEBAR_MAX = 600
const SIDEBAR_DEFAULT = 266

function loadSidebarWidth(): number {
  const saved = localStorage.getItem('sidebar-width')
  if (saved) {
    const n = parseInt(saved, 10)
    if (!isNaN(n)) return Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, n))
  }
  return SIDEBAR_DEFAULT
}

const sidebarWidth = ref(loadSidebarWidth())

let resizeStartX = 0
let resizeStartWidth = 0

function startResize(e: MouseEvent) {
  resizeStartX = e.clientX
  resizeStartWidth = sidebarWidth.value
  document.addEventListener('mousemove', doResize)
  document.addEventListener('mouseup', stopResize)
}

function doResize(e: MouseEvent) {
  const dx = e.clientX - resizeStartX
  const w = Math.max(SIDEBAR_MIN, Math.min(SIDEBAR_MAX, resizeStartWidth + dx))
  sidebarWidth.value = w
}

function stopResize() {
  document.removeEventListener('mousemove', doResize)
  document.removeEventListener('mouseup', stopResize)
  localStorage.setItem('sidebar-width', String(sidebarWidth.value))
}

function resetSidebarWidth() {
  sidebarWidth.value = SIDEBAR_DEFAULT
  localStorage.setItem('sidebar-width', String(SIDEBAR_DEFAULT))
}

onUnmounted(() => {
  document.removeEventListener('mousemove', doResize)
  document.removeEventListener('mouseup', stopResize)
})

onMounted(async () => {
  try {
    connections.value = await api.listConnections()
  } catch { /* ignore */ }
})

function changeLocale() {
  localStorage.setItem('locale', locale.value)
}

function logout() {
  router.push('/')
}

function isLoggedIn(): boolean {
  return !!route.params.connection
}

interface Crumb {
  label: string
  to?: string
  active?: boolean
}

const breadcrumbs = computed(() => {
  const items: Crumb[] = []
  const p = route.params as Record<string, string>
  const conn = p.connection
  const db = p.db
  const tbl = p.table
  const name = route.name as string

  if (conn) {
    const connInfo = connections.value.find(c => c.name === conn)
    const type = connInfo ? connInfo.db_type.toLowerCase() : ''
    if (type) {
      items.push({ label: type, to: '/' })
    }
    items.push({ label: conn, to: `/browse/${encodeURIComponent(conn)}` })
  }
  if (db) {
    items.push({ label: db, to: `/browse/${encodeURIComponent(conn)}/${encodeURIComponent(db)}` })
  }
  if (tbl) {
    items.push({ label: tbl, to: `/browse/${encodeURIComponent(conn)}/${encodeURIComponent(db)}/${encodeURIComponent(tbl)}` })
  }

  const pageLabels: Record<string, string> = {
    TableStructure: t('app.structure'),
    AlterTable: t('app.alter'),
    Indexes: t('app.indexes'),
    InsertData: t('app.insert'),
    DatabaseList: t('app.databases'),
    CreateTable: t('app.createTable'),
    AlterDatabase: t('app.alterDatabase'),
    CreateDatabase: t('app.createDatabase'),
    Export: t('app.export'),
    PrivilegesList: t('app.privileges'),
    CreateUser: t('app.createUser'),
    ProcessList: t('app.processList'),
    Variables: t('app.variables'),
    Status: t('app.status'),
    QueryConn: t('app.sqlCommand'),
  }

  if (pageLabels[name]) {
    items.push({ label: pageLabels[name], active: true })
  } else if (items.length > 0) {
    items[items.length - 1].active = true
  }

  return items
})
</script>

<template>
  <div class="app" :style="{ '--adminer-sidebar-width': sidebarWidth + 'px' }">
    <div class="top-bar">
      <div class="top-bar-lang">
        <span>{{ $t('app.language') }}:&nbsp;</span>
        <SearchableSelect v-model="locale" :options="[{value:'ZhCn',label:$t('app.zhCn')},{value:'En',label:$t('app.en')}]" @change="changeLocale" style="width:80px" />
      </div>
      <div class="top-bar-breadcrumb">
        <div class="breadcrumb" v-if="breadcrumbs.length > 0">
          <template v-for="(cr, i) in breadcrumbs" :key="i">
            <span class="separator" v-if="i > 0">&raquo;</span>
            <a v-if="cr.to" :href="cr.to" @click.prevent="cr.to && router.push(cr.to)">{{ cr.label }}</a>
            <strong v-else-if="cr.active">{{ cr.label }}</strong>
            <span v-else>{{ cr.label }}</span>
          </template>
        </div>
      </div>
      <div class="top-bar-right"></div>
    </div>
    <div class="layout-container">
      <div class="sidebar-area">
        <Sidebar />
      </div>
      <div class="sidebar-resizer" @mousedown.prevent="startResize" @dblclick="resetSidebarWidth" />
      <div class="content-area">
        <router-view />
      </div>
    </div>
  </div>
</template>

<style>
@import './assets/main.css';
</style>
