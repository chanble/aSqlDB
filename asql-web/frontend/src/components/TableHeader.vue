<script setup lang="ts">
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

defineProps<{
  tableName: string
  connection: string
  database: string
  showActions?: boolean
}>()

const emit = defineEmits<{
  (e: 'new-item'): void
  (e: 'export'): void
  (e: 'show-structure'): void
  (e: 'sql'): void
}>()
</script>

<template>
  <div class="table-header">
    <h2 class="table-title">{{ tableName || database }}</h2>
    <div v-if="showActions !== false && tableName" class="table-actions">
      <button class="button is-small" @click="emit('new-item')">
        <span class="icon"><i class="mdi mdi-plus" /></span>
        <span>{{ $t('tableHeader.newItem') }}</span>
      </button>
      <button class="button is-small" @click="emit('export')">
        <span class="icon"><i class="mdi mdi-export-variant" /></span>
        <span>{{ $t('tableHeader.export') }}</span>
      </button>
      <button class="button is-small" @click="emit('show-structure')">
        <span class="icon"><i class="mdi mdi-table" /></span>
        <span>{{ $t('tableHeader.structure') }}</span>
      </button>
      <button class="button is-small" @click="emit('sql')">
        <span class="icon"><i class="mdi mdi-code-tags" /></span>
        <span>{{ $t('tableHeader.sql') }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.table-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-bottom: 12px;
  background: #ddeeff;
  padding: 8px 12px;
  border-bottom: 1px solid #000;
}

.table-title {
  font-size: 150%;
  font-weight: normal;
  margin: 0;
  color: #000;
}

.table-actions {
  display: flex;
  gap: 6px;
}
</style>
