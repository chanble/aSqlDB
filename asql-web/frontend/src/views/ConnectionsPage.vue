<script setup lang="ts">
import { ref, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { api } from '../api'
import type { Connection, ConnectionParams } from '../types'

const { t } = useI18n()

const connections = ref<Connection[]>([])
const loading = ref(false)
const showModal = ref(false)
const editName = ref('')
const editUrl = ref('')
const error = ref('')

onMounted(async () => {
  await load()
})

async function load() {
  loading.value = true
  try {
    connections.value = await api.listConnections()
  } catch { /* ignore */ }
  loading.value = false
}

function openAdd() {
  editName.value = ''
  editUrl.value = ''
  error.value = ''
  showModal.value = true
}

function closeModal() {
  showModal.value = false
}

function urlToParams(url: string): ConnectionParams {
  const protocolEnd = url.indexOf('://')
  const scheme = protocolEnd === -1 ? '' : url.slice(0, protocolEnd)
  let rest = protocolEnd === -1 ? url : url.slice(protocolEnd + 3)

  if (scheme === 'sqlite') {
    return { type: 'Sqlite', path: rest.replace(/^\/+/, '') }
  }

  let host = ''
  let port = 0
  let user = ''
  let password: string | null = null
  let database: string | null = null

  const atIdx = rest.indexOf('@')
  if (atIdx !== -1) {
    const creds = rest.slice(0, atIdx)
    const afterAt = rest.slice(atIdx + 1)
    const slashIdx = afterAt.indexOf('/')
    if (slashIdx !== -1) {
      const hostPart = afterAt.slice(0, slashIdx)
      database = afterAt.slice(slashIdx + 1) || null
      const colIdx = hostPart.indexOf(':')
      if (colIdx !== -1) {
        host = hostPart.slice(0, colIdx)
        port = parseInt(hostPart.slice(colIdx + 1)) || 0
      } else {
        host = hostPart
      }
    } else {
      host = afterAt
    }
    const [u = '', p = ''] = creds.split(':')
    user = u
    password = p || null
  } else {
    host = rest
  }

  if (scheme === 'mysql') {
    return { type: 'MySql', host, port: port || 3306, user, password, database }
  }
  return { type: 'Postgres', host, port: port || 5432, user, password, database }
}

async function save() {
  if (!editName.value || !editUrl.value) {
    error.value = t('connections.validationError')
    return
  }
  error.value = ''
  try {
    const params = urlToParams(editUrl.value)
    await api.addConnection({ name: editName.value, params })
    showModal.value = false
    await load()
  } catch (e: any) {
    error.value = e.message
  }
}

async function remove(name: string) {
  try {
    await api.removeConnection(name)
    await load()
  } catch (e: any) {
    error.value = e.message
  }
}
</script>

<template>
  <div>
    <div style="display:flex;align-items:center;justify-content:space-between;margin-bottom:16px">
      <h2 class="page-title">{{ $t('connections.title') }}</h2>
      <button class="button is-primary" @click="openAdd">
        <span class="icon"><i class="mdi mdi-plus" /></span>
        <span>{{ $t('connections.add') }}</span>
      </button>
    </div>

    <div v-if="loading" style="padding:20px;text-align:center">
      <span class="icon"><i class="mdi mdi-loading mdi-spin" /></span>
      {{ $t('common.loading') }}
    </div>

    <table v-else class="table is-striped is-hoverable is-narrowed is-fullwidth">
      <thead>
        <tr>
          <th>{{ $t('connections.name') }}</th>
          <th>{{ $t('connections.url') }}</th>
          <th>{{ $t('connections.type') }}</th>
          <th style="width:100px">{{ $t('connections.actions') }}</th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="c in connections" :key="c.name">
          <td>{{ c.name }}</td>
          <td>{{ c.url }}</td>
          <td>{{ c.db_type }}</td>
          <td>
            <button class="button is-small is-danger" @click="remove(c.name)">
              <span class="icon"><i class="mdi mdi-delete" /></span>
              <span>{{ $t('connections.delete') }}</span>
            </button>
          </td>
        </tr>
      </tbody>
    </table>

    <!-- Modal -->
    <div :class="['modal', { 'is-active': showModal }]">
      <div class="modal-background" @click="closeModal" />
      <div class="modal-card" style="width:480px">
        <header class="modal-card-head">
          <p class="modal-card-title">{{ $t('connections.add') }}</p>
          <button class="delete" aria-label="close" @click="closeModal" />
        </header>
        <section class="modal-card-body">
          <form @submit.prevent="save">
            <div class="field">
              <label class="label">{{ $t('connections.name') }}</label>
              <div class="control">
                <input v-model="editName" class="input" type="text" :placeholder="$t('connections.namePlaceholder')" required>
              </div>
            </div>
            <div class="field">
              <label class="label">{{ $t('connections.url') }}</label>
              <div class="control">
                <input
                  v-model="editUrl"
                  class="input"
                  type="text"
                  :placeholder="$t('connections.urlPlaceholder')"
                  required
                >
              </div>
            </div>
            <p v-if="error" class="has-text-danger">{{ error }}</p>
          </form>
        </section>
        <footer class="modal-card-foot">
          <button class="button" @click="closeModal">{{ $t('connections.cancel') }}</button>
          <button class="button is-primary" @click="save">{{ $t('connections.save') }}</button>
        </footer>
      </div>
    </div>
  </div>
</template>

<style scoped>
.page-title {
  font-size: 150%;
  font-weight: normal;
  margin: 0;
}
</style>
