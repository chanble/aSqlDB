<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { api } from '../api'

const route = useRoute()
const { t } = useI18n()
const connection = computed(() => route.params.connection as string)
const status = ref<{ variable_name: string; value: string }[]>([])
const loading = ref(true)
const search = ref('')

onMounted(async () => {
  await loadStatus()
  loading.value = false
})

async function loadStatus() {
  if (!connection.value) return
  try {
    const result = await api.status(connection.value)
    status.value = result.data.map((r: any) => ({
      variable_name: r.name || '',
      value: r.value ?? '',
    }))
  } catch { /* ignore */ }
}

const filtered = computed(() => {
  if (!search.value) return status.value
  const q = search.value.toLowerCase()
  return status.value.filter(v =>
    v.variable_name.toLowerCase().includes(q) || String(v.value).toLowerCase().includes(q)
  )
})
</script>

<template>
  <div>
    <div class="page-header">{{ $t('status.title') }}</div>
    <div class="page-content">
      <fieldset style="display:inline-block;margin-bottom:12px">
        <legend>{{ $t('status.search') }}</legend>
        <div class="fieldset-content">
          <input v-model="search" type="text" style="width:200px" placeholder="Status name or value">
        </div>
      </fieldset>

      <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <table v-else>
        <thead>
          <tr>
            <th>{{ $t('status.name') }}</th>
            <th>{{ $t('status.value') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(s, i) in filtered" :key="i">
            <td>{{ s.variable_name }}</td>
            <td>{{ s.value }}</td>
          </tr>
        </tbody>
      </table>

      <div v-if="!loading && filtered.length === 0" style="padding:20px;text-align:center;color:#999">
        No status variables found
      </div>
    </div>
  </div>
</template>
