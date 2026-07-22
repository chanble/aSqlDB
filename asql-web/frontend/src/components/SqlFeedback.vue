<script setup lang="ts">
import { ref } from 'vue'
import { useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

export interface SqlFeedbackItem {
  label: string
  success: boolean
  sql: string
  duration: number
  time?: string
}

const props = defineProps<{
  items: SqlFeedbackItem[]
  connection?: string
}>()

const emit = defineEmits<{
  (e: 'remove', index: number): void
}>()

const router = useRouter()
const expanded = ref<Set<number>>(new Set())

function toggle(index: number) {
  const s = new Set(expanded.value)
  if (s.has(index)) s.delete(index)
  else s.add(index)
  expanded.value = s
}

function formatDuration(ms: number): string {
  if (ms >= 1000) {
    return (ms / 1000).toFixed(3) + ' s'
  }
  return ms + ' ms'
}

function edit(index: number) {
  const item = props.items[index]
  const query = { sql: item.sql }
  if (props.connection) {
    router.push({ name: 'QueryConn', params: { connection: props.connection }, query })
  } else {
    router.push({ name: 'Query', query })
  }
  emit('remove', index)
}
</script>

<template>
  <div v-if="items.length > 0" class="sql-feedback">
    <div
      v-for="(item, index) in items"
      :key="index"
      :class="['message', item.success ? 'success' : 'error']"
    >
      <b>{{ item.label }}</b>: {{ item.success ? $t('sqlFeedback.ok') : $t('sqlFeedback.error') }}<br>
      <span class="time">{{ item.time }}</span>
      <a href="#" class="toggle" @click.prevent="toggle(index)">{{ $t('sqlFeedback.sqlCommand') }}</a>
      <div v-show="expanded.has(index)">
        <pre><code>{{ item.sql }}</code></pre>
        <span class="time">({{ formatDuration(item.duration) }})</span>
        <p><a href="#" @click.prevent="edit(index)">{{ $t('sqlFeedback.edit') }}</a></p>
      </div>
    </div>
  </div>
</template>
