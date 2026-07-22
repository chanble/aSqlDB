<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from 'vue-i18n'

const { t } = useI18n()

const props = defineProps<{
  total: number
  current: number
  perPage: number
}>()

const emit = defineEmits<{
  (e: 'update:current', page: number): void
  (e: 'load-more'): void
}>()

const totalPages = computed(() => Math.max(1, Math.ceil(props.total / props.perPage)))

const pages = computed(() => {
  const tp = totalPages.value
  const cur = props.current
  const arr: number[] = []
  const start = Math.max(1, cur - 2)
  const end = Math.min(tp, cur + 2)
  for (let i = start; i <= end; i++) arr.push(i)
  return arr
})

function rangeStart() {
  return (props.current - 1) * props.perPage + 1
}

function rangeEnd() {
  return Math.min(props.current * props.perPage, props.total)
}
</script>

<template>
  <div class="pagination-bar">
    <span class="page-info">
      {{ $t('pagination.showing', { start: rangeStart(), end: rangeEnd(), total: props.total }) }}
    </span>

    <nav class="pagination is-small" role="navigation" aria-label="pagination">
      <a
        class="pagination-previous"
        :disabled="current <= 1"
        @click.prevent="emit('update:current', current - 1)"
      >
        {{ $t('pagination.previous') }}
      </a>
      <a
        class="pagination-next"
        :disabled="current >= totalPages"
        @click.prevent="emit('update:current', current + 1)"
      >
        {{ $t('pagination.next') }}
      </a>
      <ul class="pagination-list">
        <li v-for="p in pages" :key="p">
          <a
            :class="['pagination-link', { 'is-current': p === current }]"
            @click.prevent="emit('update:current', p)"
          >
            {{ p }}
          </a>
        </li>
      </ul>
    </nav>

    <span class="load-more-link" @click="emit('load-more')">{{ $t('pagination.loadMore') }}</span>
  </div>
</template>

<style scoped>
.pagination-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  margin-top: 12px;
  flex-wrap: wrap;
  gap: 8px;
}

.page-info {
  color: #555;
  font-size: 13px;
  white-space: nowrap;
}

.load-more-link {
  color: blue;
  cursor: pointer;
  font-size: 13px;
  white-space: nowrap;
}

.load-more-link:hover {
  color: red;
  text-decoration: underline;
}
</style>
