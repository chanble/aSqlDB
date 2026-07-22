<style scoped>
.crumb-type {
  display: inline-block;
  font-size: 10px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.03em;
  padding: 0 6px;
  margin-left: 6px;
  border-radius: 4px;
  line-height: 18px;
  vertical-align: middle;
  color: #fff;
}

.crumb-type.mysql   { background: #4479a1; }
.crumb-type.postgres { background: #336791; }
.crumb-type.sqlite  { background: #003b57; }
.crumb-type.mssql   { background: #cc2927; }
.crumb-type.oracle  { background: #c74634; }
.crumb-type.default { background: #6b7280; }
</style>

<script setup lang="ts">
import { computed, ref, onMounted, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { api } from '../api'
import type { Connection } from '../types'

const { t } = useI18n()
const route = useRoute()
const connections = ref<Connection[]>([])

function load() {
  api.listConnections()
    .then(list => { connections.value = list })
    .catch(() => {})
}

onMounted(load)
watch(() => route.params.connection, () => {
  if (route.params.connection) load()
})

function dbTypeClass(dbType: string | undefined): string {
  if (!dbType) return 'default'
  const t = dbType.toLowerCase()
  if (t.startsWith('mysql')) return 'mysql'
  if (t.startsWith('postgres')) return 'postgres'
  if (t.includes('sqlite')) return 'sqlite'
  if (t.startsWith('mssql') || t.includes('sqlserver')) return 'mssql'
  if (t.startsWith('oracle')) return 'oracle'
  return 'default'
}

const crumbs = computed(() => {
  const items: { label: string; to?: string; typeLabel?: string }[] = []
  const params = route.params as Record<string, string>

  if (route.path === '/') {
    items.push({ label: t('breadcrumb.home') })
    return items
  }

  items.push({ label: t('breadcrumb.home'), to: '/' })

  if (route.path.startsWith('/connections')) {
    items.push({ label: t('breadcrumb.connections') })
  } else if (params.connection) {
    const connName = params.connection
    const connInfo = connections.value.find(c => c.name === connName)
    const dbType = connInfo?.db_type

    items.push({ label: t('breadcrumb.connection'), to: '/connections' })
    items.push({
      label: connName,
      to: `/browse/${encodeURIComponent(connName)}`,
      typeLabel: dbType,
    })
    if (params.db) {
      items.push({
        label: params.db,
        to: `/browse/${encodeURIComponent(connName)}/${encodeURIComponent(params.db)}`,
      })
    }
    if (params.table) {
      items.push({ label: params.table })
    }
  } else if (route.path.startsWith('/query')) {
    items.push({ label: t('breadcrumb.sql') })
  } else if (route.path.startsWith('/settings')) {
    items.push({ label: t('breadcrumb.settings') })
  }
  return items
})
</script>

<template>
  <nav class="breadcrumb" aria-label="breadcrumbs">
    <ul>
      <li v-for="(c, i) in crumbs" :key="i" :class="{ 'is-active': !c.to }">
        <router-link v-if="c.to" :to="c.to">{{ c.label }}</router-link>
        <span v-else>
          {{ c.label }}
          <span v-if="c.typeLabel" :class="['crumb-type', dbTypeClass(c.typeLabel)]">{{ c.typeLabel }}</span>
        </span>
      </li>
    </ul>
  </nav>
</template>
