<script setup lang="ts">
import { ref, computed, onMounted, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import SearchableSelect from '../components/SearchableSelect.vue'
import { api } from '../api'
import type { Connection, ConnectionParams, SystemInfo } from '../types'

const { t } = useI18n()

const router = useRouter()
const route = useRoute()
const status = ref('loading')
const connections = ref<Connection[]>([])
const error = ref('')
const success = ref('')
const drivers = ref<(SystemInfo & { value: string; label: string })[]>([])

const driver = ref('')
const connectionName = ref('')
const loggingIn = ref(false)

const host = ref('')
const port = ref<number>(3306)
const username = ref('')
const password = ref('')
const database = ref('')
const path = ref('')

const selectedSystem = computed(() => drivers.value.find(d => d.value === driver.value))

const params = computed<ConnectionParams | null>(() => {
  if (!selectedSystem.value) return null
  return { ...selectedSystem.value.params } as ConnectionParams
})

function initParams() {
  const p = params.value
  if (!p) return
  if (p.type === 'MySql' || p.type === 'Postgres') {
    host.value = p.host
    port.value = p.port
    username.value = p.user
    password.value = p.password || ''
    database.value = p.database || ''
  } else if (p.type === 'Sqlite') {
    path.value = p.path
  }
}

onMounted(async () => {
  try {
    const h = await api.health()
    status.value = h.status
    const systems = await api.listSystems()
    drivers.value = systems.map(s => ({ ...s, label: s.label }))
    if (systems.length > 0) {
      driver.value = systems[0].value
    }
    connections.value = await api.listConnections()
  } catch {
    status.value = 'error'
  }
})

watch(driver, () => {
  host.value = ''
  port.value = 3306
  username.value = ''
  password.value = ''
  database.value = ''
  path.value = ''
  initParams()
})

function buildParams(): ConnectionParams | null {
  const p = params.value
  if (!p) return null
  if (p.type === 'MySql' || p.type === 'Postgres') {
    return {
      type: p.type,
      host: host.value,
      port: port.value,
      user: username.value,
      password: password.value || null,
      database: database.value || null,
    } as ConnectionParams
  } else if (p.type === 'Sqlite') {
    return {
      type: 'Sqlite',
      path: path.value,
    } as ConnectionParams
  }
  return null
}

async function doLogin() {
  error.value = ''
  success.value = ''
  const p = buildParams()
  if (!p) {
    error.value = t('home.pleaseSelect')
    return
  }
  const connName = connectionName.value || (p.type === 'Sqlite' ? path.value : database.value ? `${username.value}@${host.value}/${database.value}` : `${username.value}@${host.value}`)
  loggingIn.value = true
  try {
    await api.addConnection({ name: connName, params: p })
    success.value = t('home.connectedTo', { name: connName })
    connections.value = await api.listConnections()
    password.value = ''
    router.push(`/browse/${encodeURIComponent(connName)}`)
  } catch (e: any) {
    error.value = e.message
  }
  loggingIn.value = false
}

function parseUrl(url: string, connName?: string) {
  if (connName) connectionName.value = connName
  const protocolEnd = url.indexOf('://')
  if (protocolEnd === -1) return
  const scheme = url.slice(0, protocolEnd)
  let rest = url.slice(protocolEnd + 3)

  if (scheme === 'sqlite') {
    driver.value = 'sqlite'
    path.value = rest.replace(/^\/+/, '')
    return
  }

  if (scheme === 'mysql') driver.value = 'mysql'
  else if (scheme === 'postgresql') driver.value = 'pgsql'

  const atIdx = rest.indexOf('@')
  if (atIdx !== -1) {
    const creds = rest.slice(0, atIdx)
    const afterAt = rest.slice(atIdx + 1)
    const slashIdx = afterAt.indexOf('/')
    if (slashIdx !== -1) {
      host.value = afterAt.slice(0, slashIdx)
      database.value = afterAt.slice(slashIdx + 1)
    } else {
      host.value = afterAt
    }
    const colIdx = host.value.indexOf(':')
    if (colIdx !== -1) {
      port.value = parseInt(host.value.slice(colIdx + 1)) || 3306
      host.value = host.value.slice(0, colIdx)
    }
    const [u = '', p = ''] = creds.split(':')
    username.value = u
    password.value = p
  } else {
    host.value = rest
  }
}

watch(() => route.query, (query) => {
  if (query.edit && query.url) {
    parseUrl(query.url as string, query.edit as string)
    if (query.db_type) {
      const t = (query.db_type as string).toUpperCase()
      if (t.includes('MYSQL')) driver.value = 'mysql'
      else if (t.includes('POSTGRESQL') || t.includes('PG')) driver.value = 'pgsql'
      else if (t.includes('SQLITE')) driver.value = 'sqlite'
    }
    router.replace('/')
  }
}, { immediate: true })
</script>

<template>
  <div>
    <div class="page-header">{{ $t('home.login') }}</div>
    <div class="page-content">
      <article v-if="status === 'error'" class="message error" style="max-width:400px">
        {{ $t('home.connectionError') }}
      </article>

      <form @submit.prevent="doLogin">
        <table class="form-table">
          <tbody>
            <tr>
              <th>{{ $t('home.system') }}</th>
              <td>
                <SearchableSelect v-model="driver" :options="drivers" style="width:200px" />
              </td>
            </tr>

            <template v-if="params?.type === 'MySql' || params?.type === 'Postgres'">
              <tr>
                <th>{{ $t('home.server') }}</th>
                <td>
                  <input v-model="host" type="text" :placeholder="$t('home.hostnamePlaceholder')" style="width:300px">
                </td>
              </tr>
              <tr>
                <th>Port</th>
                <td>
                  <input v-model.number="port" type="number" style="width:100px">
                </td>
              </tr>
              <tr>
                <th>{{ $t('home.username') }}</th>
                <td>
                  <input v-model="username" type="text" :placeholder="$t('home.rootPlaceholder')" autocomplete="username" style="width:300px">
                </td>
              </tr>
              <tr>
                <th>{{ $t('home.password') }}</th>
                <td>
                  <input v-model="password" type="password" autocomplete="current-password" style="width:300px">
                </td>
              </tr>
              <tr>
                <th>{{ $t('home.database') }}</th>
                <td>
                  <input v-model="database" type="text" :placeholder="$t('home.dbPlaceholder')" style="width:300px">
                </td>
              </tr>
            </template>

            <template v-else-if="params?.type === 'Sqlite'">
              <tr>
                <th>File Path</th>
                <td>
                  <input v-model="path" type="text" placeholder="/path/to/database.db" style="width:300px">
                </td>
              </tr>
            </template>
          </tbody>
        </table>

        <div style="margin-top:8px">
          <button type="submit" :disabled="loggingIn">{{ $t('home.loginBtn') }}</button>
        </div>
      </form>

      <article v-if="error" class="message error" style="margin-top:12px;max-width:400px">
        {{ error }}
      </article>
      <article v-if="success" class="message success" style="margin-top:12px;max-width:400px">
        {{ success }}
      </article>
    </div>
  </div>
</template>
