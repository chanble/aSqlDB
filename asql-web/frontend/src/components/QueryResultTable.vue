<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  columns: string[]
  rows: Record<string, any>[]
  duration?: number
  success: boolean
  error?: string
  title?: string
}>()
</script>

<template>
  <div class="query-result">
    <div v-if="!success" class="message error">
      <code>{{ error }}</code>
    </div>
    <div v-else-if="columns.length">
      <div class="result-info">
        {{ $t('queryResult.rows', { count: rows.length }) }}{{ duration !== undefined ? ' ' + $t('queryResult.durationMs', { time: duration }) : '' }}
      </div>
      <table>
        <thead>
          <tr>
            <th v-for="col in columns" :key="col">{{ col }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(row, ri) in rows" :key="ri">
            <td v-for="col in columns" :key="col">
              {{ row[col] !== null && row[col] !== undefined ? row[col] : '' }}<span v-if="row[col] === null || row[col] === undefined" class="null-value">{{ $t('queryResult.null') }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
    <div v-else class="message success">
      {{ $t('queryResult.success') }}{{ duration !== undefined ? ' ' + $t('queryResult.durationMs', { time: duration }) : '' }}
    </div>
  </div>
</template>

<style scoped>
.result-info {
  font-size: 12px;
  color: #666;
  margin-bottom: 4px;
}

.null-value {
  color: #999;
  font-style: italic;
  opacity: 0.7;
}
td {
  white-space: pre-wrap;
}
</style>
