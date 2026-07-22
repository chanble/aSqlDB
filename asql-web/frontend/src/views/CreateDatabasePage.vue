<script setup lang="ts">
import { ref, computed, inject, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { api } from '../api'
import SqlFeedback from '../components/SqlFeedback.vue'
import SearchableSelect from '../components/SearchableSelect.vue'
import type { SqlFeedbackItem } from '../components/SqlFeedback.vue'
import type { Ref } from 'vue'

const route = useRoute()
const router = useRouter()

const { t } = useI18n()

const dbListRefreshKey = inject<Ref<number>>('dbListRefreshKey')

const connection = computed(() => route.params.connection as string)
const dbName = ref('')
const collation = ref('')
const collations = ref<Array<{ charset: string; collations: string[] }>>([])
const saving = ref(false)
const feedbackItems = ref<SqlFeedbackItem[]>([])

const collationOptions = computed(() => {
  const opts: { value: string; label: string }[] = [{ value: '', label: '(collation)' }]
  for (const group of collations.value) {
    for (const item of group.collations) {
      opts.push({ value: item, label: item })
    }
  }
  return opts
})

onMounted(async () => {
  try {
    collations.value = await api.charsets(connection.value)
  } catch { /* ignore */ }
})

function pad(n: number): string {
  return n.toString().padStart(2, '0')
}

async function save() {
  if (!dbName.value.trim()) {
    const d = new Date()
    const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
    feedbackItems.value = [{
      label: connection.value,
      success: false,
      sql: 'Database name is required',
      duration: 0,
      time,
    }]
    return
  }

  saving.value = true

  try {
    const result = await api.createDatabase(connection.value, dbName.value, undefined, collation.value || undefined)
    const d = new Date()
    const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
    feedbackItems.value = [{
      label: `${connection.value}.${dbName.value}`,
      success: true,
      sql: result.sql || '',
      duration: result.duration_ms ?? 0,
      time,
    }]
    dbListRefreshKey!.value++
    setTimeout(() => {
      router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(dbName.value)}`)
    }, 1000)
  } catch (e: any) {
    const d = new Date()
    const time = `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
    feedbackItems.value = [{
      label: connection.value,
      success: false,
      sql: '(unknown)',
      duration: 0,
      time,
    }]
  }
  saving.value = false
}

function clearForm() {
  dbName.value = ''
  collation.value = ''
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('createDatabase.title') }}</div>
    <div class="page-content">
      <SqlFeedback :items="feedbackItems" :connection="connection" @remove="(i: number) => feedbackItems.splice(i, 1)" />

      <form @submit.prevent="save">
        <table class="form-table" style="width:auto">
          <tbody>
            <tr>
              <td>
                <input v-model="dbName" type="text" style="width:200px" :placeholder="$t('createDatabase.name')">
                <SearchableSelect v-model="collation" :options="collationOptions" :placeholder="$t('createDatabase.collation')" style="margin-left:4px;min-width:200px" />
                <button type="submit" :disabled="saving" style="margin-left:4px">{{ $t('createDatabase.create') }}</button>
                <button type="button" @click="clearForm" style="margin-left:4px;font-size:16px">+</button>
              </td>
            </tr>
          </tbody>
        </table>
      </form>
    </div>
  </div>
</template>
