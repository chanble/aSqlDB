import { ref, watch } from 'vue'

const STORAGE_KEY = 'asql-query-history'
const MAX_ITEMS = 500

export interface HistoryEntry {
  time: string
  sql: string
  duration: string
  success: boolean
  connection?: string
}

const saved = localStorage.getItem(STORAGE_KEY)
const history = ref<HistoryEntry[]>(saved ? JSON.parse(saved) : [])

watch(history, () => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(history.value))
}, { deep: true })

export function useQueryHistory() {
  function addEntry(entry: HistoryEntry) {
    history.value.unshift(entry)
    if (history.value.length > MAX_ITEMS) {
      history.value.pop()
    }
  }

  function clearHistory() {
    history.value = []
    localStorage.removeItem(STORAGE_KEY)
  }

  return { history, addEntry, clearHistory }
}
