<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { api } from '../api'

const route = useRoute()
const { t } = useI18n()
const connection = computed(() => route.params.connection as string)
const processes = ref<any[]>([])
const loading = ref(true)
const selected = ref<Set<number>>(new Set())

onMounted(async () => {
  await loadProcesses()
  loading.value = false
})

async function loadProcesses() {
  if (!connection.value) return
  try {
    const result = await api.processList(connection.value)
    processes.value = result.data
  } catch { /* ignore */ }
}

function toggleSelect(id: number) {
  const s = new Set(selected.value)
  if (s.has(id)) s.delete(id)
  else s.add(id)
  selected.value = s
}

function toggleSelectAll() {
  if (selected.value.size === processes.value.length) {
    selected.value = new Set()
  } else {
    selected.value = new Set(processes.value.map(p => p.id))
  }
}

async function killSelected() {
  if (selected.value.size === 0) return
  try {
    const pids = Array.from(selected.value).map(String)
    await api.killProcesses(connection.value, pids)
  } catch { /* ignore */ }
  selected.value = new Set()
  await loadProcesses()
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('processList.title') }}</div>
    <div class="page-content">
      <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <template v-else>
        <table style="width:auto">
          <thead>
            <tr>
              <th style="width:30px"><input type="checkbox" :checked="selected.size === processes.length && processes.length > 0" @change="toggleSelectAll"></th>
              <th>{{ $t('processList.id') }}</th>
              <th>{{ $t('processList.user') }}</th>
              <th>{{ $t('processList.host') }}</th>
              <th>{{ $t('processList.db') }}</th>
              <th>{{ $t('processList.command') }}</th>
              <th>{{ $t('processList.time') }}</th>
              <th>{{ $t('processList.state') }}</th>
              <th>{{ $t('processList.info') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(p, i) in processes" :key="i">
              <td><input type="checkbox" :checked="selected.has(p.id)" @change="toggleSelect(p.id)"></td>
              <td>{{ p.id }}</td>
              <td>{{ p.user }}</td>
              <td>{{ p.host }}</td>
              <td>{{ p.db || '' }}</td>
              <td>{{ p.command }}</td>
              <td>{{ p.time }}</td>
              <td>{{ p.state }}</td>
              <td><code>{{ p.info }}</code></td>
            </tr>
          </tbody>
        </table>

        <div style="margin-top:8px;font-size:13px">
          {{ $t('processList.total', { count: processes.length }) }}
        </div>

        <div v-if="selected.size > 0" style="margin-top:8px">
          <button class="danger" @click="killSelected">{{ $t('processList.kill') }}</button>
        </div>
      </template>
    </div>
  </div>
</template>
