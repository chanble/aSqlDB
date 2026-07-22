<script setup lang="ts">
import { ref, computed, onMounted } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import { api } from '../api'
import SqlFeedback from '../components/SqlFeedback.vue'
import SearchableSelect from '../components/SearchableSelect.vue'
import type { SqlFeedbackItem } from '../components/SqlFeedback.vue'


const route = useRoute()
const router = useRouter()

const { t } = useI18n()

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)

const dbName = ref('')
const collation = ref('utf8mb4_general_ci')
const collations = ref<Array<{ charset: string; collations: string[] }>>([])
const saving = ref(false)
const loading = ref(true)
const feedbackItems = ref<SqlFeedbackItem[]>([])

const collationOptions = computed(() => {
  const opts: { value: string; label: string }[] = []
  for (const group of collations.value) {
    for (const item of group.collations) {
      opts.push({ value: item, label: item })
    }
  }
  return opts
})

onMounted(async () => {
  dbName.value = database.value
  await Promise.all([loadCollation(), loadCharsets()])
  loading.value = false
})

async function loadCollation() {
  try {
    const escapedDb = database.value.replace(/'/g, "''")
    const sql = `SELECT DEFAULT_COLLATION_NAME FROM information_schema.SCHEMATA WHERE SCHEMA_NAME = '${escapedDb}'`
    const results = await api.executeQuery(connection.value, sql)
    if (results.length > 0 && results[0].success) {
      const rows = results[0].data?.rows
      if (Array.isArray(rows) && rows.length > 0) {
        collation.value = rows[0].DEFAULT_COLLATION_NAME || 'utf8mb4_general_ci'
      }
    }
  } catch { /* ignore */ }
}

async function loadCharsets() {
  try {
    collations.value = await api.charsets(connection.value)
  } catch { /* ignore */ }
}

async function save() {
  saving.value = true
  const sql = `ALTER DATABASE \`${database.value}\` COLLATE ${collation.value}`
  try {
    await api.alterDatabase(connection.value, database.value, undefined, collation.value)
    const d = new Date()
    const time = `${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}:${d.getSeconds().toString().padStart(2, '0')}`
    feedbackItems.value = [{ label: `${connection.value}.${database.value}`, success: true, sql, duration: 0, time }]
  } catch {
    const d = new Date()
    const time = `${d.getHours().toString().padStart(2, '0')}:${d.getMinutes().toString().padStart(2, '0')}:${d.getSeconds().toString().padStart(2, '0')}`
    feedbackItems.value = [{ label: `${connection.value}.${database.value}`, success: false, sql, duration: 0, time }]
  }
  saving.value = false
}

async function dropDatabase() {
  if (!confirm(`DROP DATABASE \`${database.value}\`?`)) return
  try {
    await api.dropDatabase(connection.value, database.value)
    router.push(`/browse/${encodeURIComponent(connection.value)}`)
  } catch (e: any) {
    alert('Error: ' + (e.message || e))
  }
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('alterDatabase.title') }}: {{ database }}</div>
    <div class="page-content">
      <SqlFeedback :items="feedbackItems" :connection="connection" @remove="(i: number) => feedbackItems.splice(i, 1)" />

      <div class="top-actions" style="margin-bottom:12px">
        <a href="#" class="active">{{ $t('alterDatabase.title') }}</a>
        <a href="#">{{ $t('alterDatabase.databaseSchema') }}</a>
        <a href="#">{{ $t('alterDatabase.privileges') }}</a>
      </div>

      <div v-if="loading" style="padding:20px;text-align:center;color:#999">Loading...</div>

      <template v-else>
        <table class="form-table" style="width:auto">
          <tbody>
            <tr>
              <td>
                <input v-model="dbName" type="text" style="width:150px">
                <SearchableSelect v-model="collation" :options="collationOptions" style="margin-left:4px;min-width:180px" />
                <button @click="save" :disabled="saving" style="margin-left:4px">{{ $t('alterDatabase.save') }}</button>
                <button class="danger" style="margin-left:4px" @click="dropDatabase">{{ $t('alterDatabase.drop') }}</button>
              </td>
            </tr>
          </tbody>
        </table>
      </template>
    </div>
  </div>
</template>
