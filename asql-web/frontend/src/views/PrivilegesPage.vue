<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { api } from '../api'
import SqlFeedback from '../components/SqlFeedback.vue'
import type { SqlFeedbackItem } from '../components/SqlFeedback.vue'

const { t } = useI18n()
const route = useRoute()
const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)

const server = ref('%')
const username = ref('')
const password = ref('')
const hashed = ref(false)
const saving = ref(false)
const feedbackItems = ref<SqlFeedbackItem[]>([])

const allPrivileges = [
  { scope: 'All privileges', name: 'ALL PRIVILEGES' },
  { scope: '', name: 'Grant option' },
  { scope: 'Server', name: 'Create user' },
  { scope: 'Server', name: 'Event' },
  { scope: 'Server', name: 'Process' },
  { scope: 'Server', name: 'Proxy' },
  { scope: 'Server', name: 'Reload' },
  { scope: 'Server', name: 'Replication client' },
  { scope: 'Server', name: 'Replication slave' },
  { scope: 'Server', name: 'Show databases' },
  { scope: 'Server', name: 'Shutdown' },
  { scope: 'Server', name: 'Super' },
  { scope: 'Server', name: 'Create tablespace' },
  { scope: 'Server', name: 'File' },
  { scope: 'Database', name: 'Create routine' },
  { scope: 'Database', name: 'Create temporary tables' },
  { scope: 'Database', name: 'Lock tables' },
  { scope: 'Table', name: 'Alter' },
  { scope: 'Table', name: 'Create' },
  { scope: 'Table', name: 'Create view' },
  { scope: 'Table', name: 'Delete' },
  { scope: 'Table', name: 'Drop' },
  { scope: 'Table', name: 'Index' },
  { scope: 'Table', name: 'Insert' },
  { scope: 'Table', name: 'References' },
  { scope: 'Table', name: 'Select' },
  { scope: 'Table', name: 'Show view' },
  { scope: 'Table', name: 'Update' },
  { scope: 'Column', name: 'Select' },
  { scope: 'Column', name: 'Insert' },
  { scope: 'Column', name: 'Update' },
  { scope: 'Column', name: 'References' },
  { scope: 'Routine', name: 'Alter routine' },
  { scope: 'Routine', name: 'Execute' },
]

const selectedPrivileges = ref<Set<string>>(new Set())

function togglePriv(name: string) {
  const s = new Set(selectedPrivileges.value)
  if (s.has(name)) s.delete(name)
  else s.add(name)
  selectedPrivileges.value = s
}

function selectAll() {
  if (selectedPrivileges.value.size === allPrivileges.length) {
    selectedPrivileges.value = new Set()
  } else {
    selectedPrivileges.value = new Set(allPrivileges.map(p => p.name))
  }
}

function pad(n: number): string {
  return n.toString().padStart(2, '0')
}

async function save() {
  if (!username.value || !password.value) {
    const d = new Date()
    const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
    feedbackItems.value = [{
      label: connection.value,
      success: false,
      sql: 'Username and password are required',
      duration: 0,
      time,
    }]
    return
  }

  saving.value = true

  const user = username.value.replace(/'/g, "''")
  const host = server.value.replace(/'/g, "''")
  const pass = password.value.replace(/'/g, "''")

  const d = new Date()
  const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`

  try {
    // Create user
    await api.createUser(connection.value, {
      username: username.value,
      host: host,
      password: pass,
    })
    let sql = 'CREATE USER'

    // Grant privileges
    if (selectedPrivileges.value.size > 0) {
      const privs = Array.from(selectedPrivileges.value)
      await api.grant(connection.value, username.value, {
        privileges: privs,
        on: `${database.value}.*`,
        host,
        with_grant_option: false,
      })
      sql = 'GRANT'
    }

    // Flush privileges
    await api.executeQuery(connection.value, 'FLUSH PRIVILEGES')

    feedbackItems.value = [{
      label: connection.value,
      success: true,
      sql,
      duration: 0,
      time,
    }]
    username.value = ''
    password.value = ''
    selectedPrivileges.value = new Set()
  } catch (e: any) {
    feedbackItems.value = [{
      label: connection.value,
      success: false,
      sql: 'Error: ' + (e.message || e),
      duration: 0,
      time,
    }]
  }
  saving.value = false
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('privileges.createUser') }}</div>
    <div class="page-content">
      <SqlFeedback :items="feedbackItems" :connection="connection" @remove="(i: number) => feedbackItems.splice(i, 1)" />

      <table class="form-table" style="width:auto;margin-bottom:12px">
        <tbody>
          <tr>
            <th>{{ $t('privileges.host') }}</th>
            <td><input v-model="server" type="text" style="width:200px"></td>
          </tr>
          <tr>
            <th>{{ $t('privileges.username') }}</th>
            <td><input v-model="username" type="text" style="width:200px"></td>
          </tr>
          <tr>
            <th>{{ $t('privileges.password') }}</th>
            <td>
              <input v-model="password" type="password" style="width:200px">
              <label style="margin-left:8px"><input v-model="hashed" type="checkbox"> {{ $t('privileges.hashed') }}</label>
            </td>
          </tr>
        </tbody>
      </table>

      <table style="width:auto">
        <thead>
          <tr>
            <th>{{ $t('privileges.title') }}</th>
            <th style="width:150px">
              <input type="text" :value="`'${database}'.*`" readonly style="width:100px;font-size:12px">
            </th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(priv, i) in allPrivileges" :key="i">
            <td>
              {{ priv.scope === 'All privileges' ? $t('privileges.allPrivileges') + ' ' : (priv.scope ? $t('privileges.scope.' + priv.scope.toLowerCase()) + ' ' : '') }}{{ priv.name === 'Grant option' ? $t('privileges.grantOption') : priv.name }}
            </td>
            <td>
              <input type="checkbox" :checked="selectedPrivileges.has(priv.name)" @change="togglePriv(priv.name)">
            </td>
          </tr>
        </tbody>
      </table>

      <div style="margin-top:8px">
        <button @click="selectAll" style="font-size:11px">
          {{ selectedPrivileges.size === allPrivileges.length ? $t('privileges.deselectAll') : $t('privileges.selectAll') }}
        </button>
      </div>

      <div style="margin-top:12px">
        <button @click="save" :disabled="saving">{{ $t('privileges.save') }}</button>
      </div>
    </div>
  </div>
</template>
