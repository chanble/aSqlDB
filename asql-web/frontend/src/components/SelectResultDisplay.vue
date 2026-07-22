<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const props = defineProps<{
  rows: Record<string, any>[]
  title?: string
}>()

const { t } = useI18n()

const columns = computed<string[]>(() => {
  if (props.rows.length > 0) {
    return Object.keys(props.rows[0])
  }
  return []
})
</script>

<template>
  <div class="select-result">
    <div v-if="title" class="result-title">{{ title }}</div>
    <div v-if="columns.length">
      <div class="result-info">
        {{ t('queryResult.rows', { count: rows.length }) }}
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
              {{ row[col] !== null && row[col] !== undefined ? row[col] : '' }}<span v-if="row[col] === null || row[col] === undefined" class="null-value">{{ t('queryResult.null') }}</span>
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.result-title {
  font-weight: 600;
  margin-bottom: 4px;
}
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
