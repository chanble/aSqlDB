<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { api } from '../api'

const route = useRoute()
const { t } = useI18n()
const connection = computed(() => route.params.connection as string)
const variables = ref<{ variable_name: string; value: string }[]>([])
const loading = ref(true)
const search = ref('')

onMounted(async () => {
  await loadVariables()
  loading.value = false
})

async function loadVariables() {
  if (!connection.value) return
  try {
    const result = await api.variables(connection.value)
    variables.value = result.data.map((r: any) => ({
      variable_name: r.name || '',
      value: r.value ?? '',
    }))
  } catch { /* ignore */ }
}

const filtered = computed(() => {
  if (!search.value) return variables.value
  const q = search.value.toLowerCase()
  return variables.value.filter(v =>
    v.variable_name.toLowerCase().includes(q) || String(v.value).toLowerCase().includes(q)
  )
})
</script>

<template>
  <div>
    <div class="page-header">{{ $t('variables.title') }}</div>
    <div class="page-content">
      <fieldset style="display:inline-block;margin-bottom:12px">
        <legend>Search</legend>
        <div class="fieldset-content">
          <input v-model="search" type="text" style="width:200px" placeholder="Variable name or value">
        </div>
      </fieldset>

      <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <table v-else>
        <thead>
          <tr>
            <th>{{ $t('variables.name') }}</th>
            <th>{{ $t('variables.value') }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(v, i) in filtered" :key="i">
            <td>{{ v.variable_name }}</td>
            <td>{{ v.value }}</td>
          </tr>
        </tbody>
      </table>

      <div v-if="!loading && filtered.length === 0" style="padding:20px;text-align:center;color:#999">
        No variables found
      </div>
    </div>
  </div>
</template>
