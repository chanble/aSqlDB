<script setup lang="ts">
import { ref, computed, watch, nextTick, onMounted, onBeforeUnmount } from 'vue'

interface Option {
  value: string
  label: string
}

const props = defineProps<{
  options: Option[]
  modelValue: string
  size?: 'small' | 'normal'
  placeholder?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [value: string]
  'change': [value: string]
}>()

const wrapper = ref<HTMLDivElement>()
const searchInput = ref<HTMLInputElement>()
const optionsEl = ref<HTMLDivElement>()
const open = ref(false)
const search = ref('')
const highlightIndex = ref(-1)
const dropUp = ref(false)

const displayText = computed(() => {
  const found = props.options.find(o => o.value === props.modelValue)
  return found ? found.label : ''
})

const filtered = computed(() => {
  if (!search.value) return props.options
  const q = search.value.toLowerCase()
  return props.options.filter(
    o => o.label.toLowerCase().includes(q) || o.value.toLowerCase().includes(q),
  )
})

watch(() => props.modelValue, () => {
  search.value = ''
})

watch(filtered, () => {
  highlightIndex.value = -1
})

function toggle() {
  open.value = !open.value
  if (open.value) {
    search.value = ''
    calcDropUp()
    nextTick(() => searchInput.value?.focus())
  }
}

function calcDropUp() {
  if (!wrapper.value) return
  const rect = wrapper.value.getBoundingClientRect()
  const spaceBelow = window.innerHeight - rect.bottom
  dropUp.value = spaceBelow < 180
}

function select(opt: Option) {
  emit('update:modelValue', opt.value)
  emit('change', opt.value)
  open.value = false
}

function onResize() {
  if (open.value) calcDropUp()
}

function onDocumentClick(e: MouseEvent) {
  if (wrapper.value && !wrapper.value.contains(e.target as Node)) {
    open.value = false
  }
}

function scrollIntoView() {
  if (!optionsEl.value) return
  const el = optionsEl.value.children[highlightIndex.value] as HTMLElement | undefined
  if (el) el.scrollIntoView({ block: 'nearest' })
}

function onKeydown(e: KeyboardEvent) {
  if (!open.value) return
  if (e.key === 'Escape') {
    open.value = false
    e.preventDefault()
    return
  }
  if (e.key === 'ArrowDown') {
    e.preventDefault()
    if (highlightIndex.value < filtered.value.length - 1) {
      highlightIndex.value++
      nextTick(scrollIntoView)
    }
    return
  }
  if (e.key === 'ArrowUp') {
    e.preventDefault()
    if (highlightIndex.value > 0) {
      highlightIndex.value--
      nextTick(scrollIntoView)
    }
    return
  }
  if (e.key === 'Enter') {
    e.preventDefault()
    if (highlightIndex.value < 0) return
    const opt = filtered.value[highlightIndex.value]
    if (opt) select(opt)
    return
  }
}

onMounted(() => {
  document.addEventListener('click', onDocumentClick, true)
  document.addEventListener('keydown', onKeydown)
  window.addEventListener('resize', onResize)
})

onBeforeUnmount(() => {
  document.removeEventListener('click', onDocumentClick, true)
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('resize', onResize)
})
</script>

<template>
  <div ref="wrapper" :class="['searchable-select', { 'is-open': open }]">
    <div
      :class="['select-trigger', { 'is-small': size === 'small' }]"
      @click.prevent="toggle"
    >
      <span v-if="displayText" class="select-value">{{ displayText }}</span>
      <span v-else class="select-placeholder">{{ placeholder }}</span>
      <span class="select-arrow">&#x25BC;</span>
    </div>
    <div v-if="open" :class="['select-dropdown', { 'drop-up': dropUp }]">
      <div class="select-search-wrap">
        <input
          ref="searchInput"
          v-model="search"
          class="select-search-input"
          type="text"
          placeholder="Search..."
        >
      </div>
      <div ref="optionsEl" class="select-options">
        <div
          v-for="(o, i) in filtered"
          :key="o.value"
          :class="['select-option', { 'is-active': o.value === modelValue, 'is-highlighted': i === highlightIndex }]"
          @mousedown.prevent="select(o)"
        >
          {{ o.label }}
        </div>
        <div v-if="!filtered.length" class="select-option is-disabled">No matches</div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.searchable-select {
  position: relative;
  display: inline-block;
}

.select-trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 4px;
  width: 100%;
  padding: 2px 4px;
  border: 1px solid var(--adminer-border);
  background: #fff;
  cursor: pointer;
  font-size: 13px;
  font-family: var(--adminer-font);
  line-height: normal;
  user-select: none;
}

.select-trigger.is-small {
  font-size: 12px;
  padding: 1px 4px;
}

.select-value {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #000;
}

.select-placeholder {
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  color: #999;
}

.select-arrow {
  font-size: 10px;
  color: #666;
  flex-shrink: 0;
  line-height: 1;
}

.is-open .select-arrow {
  color: #000;
}

.select-dropdown {
  position: absolute;
  top: 100%;
  left: 0;
  z-index: 200;
  min-width: 100%;
  border: 1px solid var(--adminer-border);
  border-top: none;
  background: #fff;
  max-height: 250px;
  display: flex;
  flex-direction: column;
  box-shadow: 0 2px 4px rgba(0,0,0,0.1);
}

.select-dropdown.drop-up {
  top: auto;
  bottom: 100%;
  border-top: 1px solid var(--adminer-border);
  border-bottom: none;
}

.select-search-wrap {
  padding: 4px;
  border-bottom: 1px solid var(--adminer-border);
  flex-shrink: 0;
}

.select-search-input {
  width: 100%;
  font-size: 12px;
  padding: 2px 4px;
  border: 1px solid var(--adminer-border);
  font-family: var(--adminer-font);
  outline: none;
  box-sizing: border-box;
  background: #fff;
  color: #000;
}

.select-search-input:focus {
  border-color: #666;
}

.select-options {
  overflow-y: auto;
  flex: 1;
}

.select-option {
  padding: 3px 6px;
  font-size: 12px;
  cursor: pointer;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  min-height: 22px;
  display: flex;
  align-items: center;
  box-sizing: border-box;
}

.select-option:hover,
.select-option.is-active,
.select-option.is-highlighted {
  background: #ddeeff;
}

.select-option.is-disabled {
  color: #999;
  cursor: default;
}

.select-option.is-disabled:hover {
  background: transparent;
}
</style>
