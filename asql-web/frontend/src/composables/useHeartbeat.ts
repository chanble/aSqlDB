import { ref, watch, onUnmounted } from 'vue'
import { api } from '../api'

export function useHeartbeat(connNameRef: import('vue').Ref<string | undefined>) {
  const timer = ref<ReturnType<typeof setInterval> | null>(null)

  function stop() {
    if (timer.value) {
      clearInterval(timer.value)
      timer.value = null
    }
  }

  function start() {
    stop()
    if (!connNameRef.value) return
    timer.value = setInterval(async () => {
      try {
        await api.pingConnection(connNameRef.value!)
      } catch {
        // silent
      }
    }, 30000)
  }

  watch(connNameRef, (name) => {
    name ? start() : stop()
  }, { immediate: true })

  onUnmounted(stop)

  return { start, stop }
}
