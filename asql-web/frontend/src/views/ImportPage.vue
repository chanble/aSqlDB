<script setup lang="ts">
import { ref, computed, watch, onMounted, onUnmounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useI18n } from 'vue-i18n'
import { api } from '../api'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)

type Mode = 'upload' | 'server'
const mode = ref<Mode>('upload')

const fileName = ref('')
const uploadedFilePath = ref('')
const serverPath = ref('')
const selectedFile = ref<File | null>(null)

const stopOnError = ref(false)
const singleTransaction = ref(false)
const loading = ref(false)
const executing = ref(false)
const errorMsg = ref('')

interface PreviewData {
  totalLines: number
  fileSize: number
  head: string
  tail: string
  omitted: number
}
const preview = ref<PreviewData | null>(null)

interface ImportError {
  index: number
  error: string
}
interface TaskStatus {
  id: string
  status: 'running' | 'completed' | 'failed' | 'cancelled'
  total: number
  current: number
  succeeded: number
  failed: number
  durationMs: number
  error: string | null
  errors: ImportError[]
  connection: string
  database: string | null
  file_name: string
  file_path: string
  total_lines: number
  file_size: number
  preview_head: string
  preview_tail: string
  preview_omitted: number
  stop_on_error: boolean
  single_transaction: boolean
  created_at: number
  finished_at: number | null
}
const task = ref<TaskStatus | null>(null)
const pollTimer = ref<number | null>(null)
const activeTaskId = ref('')
const cancelling = ref(false)

const isTaskView = computed(() => !!route.query.task)

const isRunning = computed(() => task.value?.status === 'running')

const progressPercent = computed(() => {
  const tsk = task.value
  if (!tsk || tsk.total <= 0) return 0
  return Math.min(100, Math.round((tsk.current / tsk.total) * 100))
})

function formatSize(bytes: number): string {
  if (bytes >= 1048576) return (bytes / 1048576).toFixed(2) + ' MB'
  if (bytes >= 1024) return (bytes / 1024).toFixed(2) + ' KB'
  return bytes + ' B'
}

function formatDuration(ms: number): string {
  if (ms >= 1000) return (ms / 1000).toFixed(2) + 's'
  return ms + 'ms'
}

function formatTime(ms: number): string {
  const d = new Date(ms)
  const pad = (n: number) => String(n).padStart(2, '0')
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`
}

interface TaskListItem {
  id: string
  connection: string
  database: string | null
  file_name: string
  file_path: string
  status: 'running' | 'completed' | 'failed' | 'cancelled'
  total: number
  total_lines: number
  file_size: number
  current: number
  succeeded: number
  failed: number
  error_count: number
  duration_ms: number
  created_at: number
  finished_at: number | null
  stop_on_error: boolean
  single_transaction: boolean
}
const taskList = ref<TaskListItem[]>([])
const hasRunningTask = computed(() => taskList.value.some(t => t.status === 'running'))

async function refreshTaskList() {
  try {
    const res = await api.listImportTasks()
    taskList.value = res.tasks
  } catch {
    // ignore list refresh errors
  }
}

function goTaskDetail(t: TaskListItem) {
  router.push({
    name: 'Import',
    params: { connection: t.connection, db: t.database || '-' },
    query: { task: t.id },
  })
}

function stopPolling() {
  if (pollTimer.value !== null) {
    window.clearInterval(pollTimer.value)
    pollTimer.value = null
  }
}

async function startPolling(taskId: string) {
  stopPolling()
  activeTaskId.value = taskId
  const poll = async () => {
    try {
      const s = await api.importStatus(taskId)
      task.value = {
        id: s.id,
        status: s.status,
        total: s.total,
        current: s.current,
        succeeded: s.succeeded,
        failed: s.failed,
        durationMs: s.duration_ms,
        error: s.error,
        errors: s.errors,
        connection: s.connection,
        database: s.database,
        file_name: s.file_name,
        file_path: s.file_path,
        total_lines: s.total_lines,
        file_size: s.file_size,
        preview_head: s.preview_head,
        preview_tail: s.preview_tail,
        preview_omitted: s.preview_omitted,
        stop_on_error: s.stop_on_error,
        single_transaction: s.single_transaction,
        created_at: s.created_at,
        finished_at: s.finished_at,
      }
      if (!preview.value && s.preview_head) {
        preview.value = {
          totalLines: s.total_lines,
          fileSize: s.file_size,
          head: s.preview_head,
          tail: s.preview_tail,
          omitted: s.preview_omitted,
        }
      }
      if (s.status !== 'running') {
        cancelling.value = false
        stopPolling()
        const q = { ...route.query }
        delete q.task
        router.replace({ query: q })
      }
    } catch (e: any) {
      stopPolling()
      cancelling.value = false
      task.value = {
        id: taskId,
        status: 'failed',
        total: 0,
        current: 0,
        succeeded: 0,
        failed: 0,
        durationMs: 0,
        error: e.message || t('import.taskNotFound'),
        errors: [],
        connection: '',
        database: null,
        file_name: '',
        file_path: '',
        total_lines: 0,
        file_size: 0,
        preview_head: '',
        preview_tail: '',
        preview_omitted: 0,
        stop_on_error: false,
        single_transaction: false,
        created_at: 0,
        finished_at: null,
      }
    }
  }
  await poll()
  pollTimer.value = window.setInterval(poll, 1000)
  refreshTaskList()
}

async function onCancel() {
  if (!activeTaskId.value) return
  cancelling.value = true
  try {
    await api.cancelImport(activeTaskId.value)
  } catch (e: any) {
    cancelling.value = false
    errorMsg.value = e.message || t('import.executeError')
  }
}

function onFileChange(e: Event) {
  const target = e.target as HTMLInputElement
  const file = target.files?.[0]
  if (!file) return
  selectedFile.value = file
  fileName.value = file.name
  preview.value = null
  task.value = null
  errorMsg.value = ''
  uploadedFilePath.value = ''
}

const currentFilePath = computed(() =>
  mode.value === 'upload' ? uploadedFilePath.value : serverPath.value.trim(),
)

const displayLabel = computed(() => {
  if (isTaskView.value && task.value) return task.value.file_name
  return mode.value === 'upload' ? fileName.value : serverPath.value
})

async function onPreview() {
  errorMsg.value = ''
  preview.value = null
  task.value = null

  if (mode.value === 'upload') {
    if (!selectedFile.value) {
      errorMsg.value = t('import.noFileSelected')
      return
    }
    loading.value = true
    try {
      const res = await api.uploadImportFile(connection.value, selectedFile.value)
      uploadedFilePath.value = res.file_path
      const previewRes = await api.importPreview(connection.value, res.file_path)
      preview.value = {
        totalLines: previewRes.total_lines,
        fileSize: previewRes.file_size,
        head: previewRes.head,
        tail: previewRes.tail,
        omitted: previewRes.omitted,
      }
    } catch (e: any) {
      errorMsg.value = e.message || t('import.previewError')
    }
    loading.value = false
  } else {
    if (!serverPath.value.trim()) {
      errorMsg.value = t('import.noFileSelected')
      return
    }
    loading.value = true
    try {
      const res = await api.importPreview(connection.value, serverPath.value.trim())
      preview.value = {
        totalLines: res.total_lines,
        fileSize: res.file_size,
        head: res.head,
        tail: res.tail,
        omitted: res.omitted,
      }
    } catch (e: any) {
      errorMsg.value = e.message || t('import.previewError')
    }
    loading.value = false
  }
}

async function onExecute() {
  errorMsg.value = ''
  task.value = null

  const filePath = currentFilePath.value
  if (!filePath) {
    errorMsg.value = t('import.noFileSelected')
    return
  }

  executing.value = true
  try {
    const res = await api.importServerFile(connection.value, filePath, {
      stopOnError: stopOnError.value,
      singleTransaction: singleTransaction.value,
      database: database.value,
      fileName: displayLabel.value || undefined,
    })
    await router.replace({ query: { ...route.query, task: res.task_id } })
  } catch (e: any) {
    errorMsg.value = e.message || t('import.executeError')
  }
  executing.value = false
}

function resetToFresh() {
  stopPolling()
  activeTaskId.value = ''
  fileName.value = ''
  uploadedFilePath.value = ''
  serverPath.value = ''
  selectedFile.value = null
  preview.value = null
  task.value = null
  errorMsg.value = ''
  cancelling.value = false
}

function onClear() {
  resetToFresh()
}

watch(
  () => route.query.task,
  (taskId) => {
    if (!taskId) {
      resetToFresh()
      refreshTaskList()
    } else if (typeof taskId === 'string') {
      startPolling(taskId)
    }
  },
)

const hasPreview = computed(() => preview.value !== null)

onMounted(() => {
  const taskId = route.query.task as string | undefined
  if (taskId) {
    startPolling(taskId)
  }
  refreshTaskList()
})

onUnmounted(() => {
  stopPolling()
})
</script>

<template>
  <div>
    <div class="page-header">{{ $t('import.title') }}: {{ database }}</div>
    <div class="page-content">
      <div v-if="!isTaskView" class="import-mode">
        <label>
          <input type="radio" value="upload" v-model="mode" @change="onClear">
          {{ $t('import.uploadFile') }}
        </label>
        <label style="margin-left: 16px">
          <input type="radio" value="server" v-model="mode" @change="onClear">
          {{ $t('import.serverPath') }}
        </label>
      </div>

      <!-- Upload mode -->
      <div v-if="!isTaskView && mode === 'upload'" class="import-input">
        <label class="file-label">
          <input type="file" accept=".sql,.txt" @change="onFileChange" style="display:none">
          <span class="file-btn">{{ $t('import.chooseFile') }}</span>
        </label>
        <span v-if="fileName" class="file-info">{{ fileName }}</span>
        <button @click="onPreview" :disabled="loading || !fileName" style="margin-left: 8px">
          {{ loading ? t('import.uploading') : t('import.preview') }}
        </button>
      </div>

      <!-- Server path mode -->
      <div v-else-if="!isTaskView" class="import-input">
        <input
          v-model="serverPath"
          type="text"
          :placeholder="$t('import.filePath')"
          style="width: 400px; font-size: 13px; padding: 2px 6px; border: 1px solid #999"
          @keyup.enter="onPreview"
        >
        <button @click="onPreview" :disabled="loading" style="margin-left: 8px">
          {{ loading ? t('import.loading') : t('import.preview') }}
        </button>
      </div>

      <article v-if="errorMsg" class="message error" style="margin-top: 12px">
        {{ errorMsg }}
      </article>

      <!-- Task detail meta (task view mode) -->
      <div v-if="isTaskView && task" class="task-meta">
        <span>{{ t('import.colFileName') }}: <strong>{{ task.file_name }}</strong></span>
        <span v-if="task.database">{{ t('import.database') }}: {{ task.database }}</span>
        <span>{{ t('import.colLines') }}: {{ task.total_lines.toLocaleString() }}</span>
        <span>{{ t('import.colSize') }}: {{ formatSize(task.file_size) }}</span>
        <span>{{ t('import.colStopOnError') }}: {{ task.stop_on_error ? '✓' : '—' }}</span>
        <span>{{ t('import.colSingleTx') }}: {{ task.single_transaction ? '✓' : '—' }}</span>
      </div>

      <!-- Preview -->
      <div v-if="hasPreview" class="preview-section">
        <div class="preview-header">
          <strong>{{ displayLabel }}</strong>
          | {{ preview!.totalLines.toLocaleString() }} {{ t('import.lines') }}
          | {{ formatSize(preview!.fileSize) }}
        </div>
        <pre class="preview-box"><code v-if="preview!.head">{{ t('import.firstLines') }}
{{ preview!.head }}</code><code v-if="preview!.tail">{{ preview!.omitted > 0 ? '\n' + t('import.linesOmitted', { count: preview!.omitted }) + '\n' : '' }}{{ t('import.lastLines') }}
{{ preview!.tail }}</code></pre>
      </div>

      <!-- Execute controls -->
      <div v-if="hasPreview && !isTaskView" class="execute-bar">
        <label class="stop-on-error">
          <input type="checkbox" v-model="stopOnError">
          {{ t('import.stopOnError') }}
        </label>
        <label class="stop-on-error" style="margin-left: 12px">
          <input type="checkbox" v-model="singleTransaction">
          {{ t('import.singleTransaction') }}
        </label>
        <button @click="onExecute" :disabled="executing || isRunning" style="margin-left: 12px">
          {{ executing || isRunning ? t('import.executing') : t('import.execute') }}
        </button>
        <button @click="onClear" style="margin-left: 8px">
          {{ t('import.clear') }}
        </button>
      </div>
      <div v-if="singleTransaction" class="tx-hint">
        {{ t('import.singleTransactionHint') }}
      </div>

      <!-- Progress -->
      <div v-if="isRunning" class="progress-section">
        <div class="progress-text">
          <span v-if="task!.total > 0">
            {{ t('import.progress', { current: task!.current, total: task!.total }) }}
            <span class="progress-stats">
              (<span class="stat-ok">{{ t('import.progressOk', { succeeded: task!.succeeded }) }}</span>
              <span class="stat-fail">{{ t('import.progressFail', { failed: task!.failed }) }}</span>)
            </span>
          </span>
          <span v-else>{{ t('import.preparing') }}</span>
          <button class="cancel-btn" @click="onCancel" :disabled="cancelling" style="margin-left: 12px">
            {{ cancelling ? t('import.cancelling') : t('import.cancel') }}
          </button>
        </div>
        <div class="progress-bar">
          <div class="progress-fill" :style="{ width: progressPercent + '%' }"></div>
        </div>
      </div>

      <!-- Results -->
      <div v-if="task && !isRunning" class="results-section">
        <div v-if="task.status === 'failed' && task.error" class="message error">
          {{ task.error }}
        </div>
        <div v-if="task.status === 'cancelled'" class="message warn">
          {{ t('import.cancelled', { succeeded: task.succeeded, failed: task.failed }) }}
        </div>
        <template v-if="task.status !== 'cancelled' && task.total > 0">
          <div :class="['message', task.failed === 0 ? 'success' : 'error']">
            <template v-if="task.failed === 0">
              {{ t('import.completed', { total: task.total, duration: formatDuration(task.durationMs) }) }}
            </template>
            <template v-else>
              {{ t('import.completedWithErrors', { succeeded: task.succeeded, failed: task.failed, duration: formatDuration(task.durationMs) }) }}
            </template>
          </div>

          <div v-if="task.errors.length > 0" class="error-list">
            <div v-for="(err, i) in task.errors" :key="i" class="error-item">
              <div class="error-index">{{ t('import.error') }} {{ i + 1 }} (#{{ err.index + 1 }})</div>
              <pre class="error-detail">{{ err.error }}</pre>
            </div>
          </div>
        </template>
      </div>

      <!-- Task list -->
      <div class="task-list-section">
        <div class="task-list-title">{{ t('import.taskList') }}</div>
        <table>
          <thead>
            <tr>
              <th>{{ t('import.colStartTime') }}</th>
              <th>{{ t('import.colFileName') }}</th>
              <th>{{ t('import.colLines') }}</th>
              <th>{{ t('import.colSize') }}</th>
              <th>{{ t('import.colStopOnError') }}</th>
              <th>{{ t('import.colSingleTx') }}</th>
              <th>{{ t('import.colStatus') }}</th>
              <th>{{ t('import.colDuration') }}</th>
              <th v-if="hasRunningTask">{{ t('import.detail') }}</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="t2 in taskList" :key="t2.id">
              <td>{{ formatTime(t2.created_at) }}</td>
              <td>{{ t2.file_name }}</td>
              <td>{{ t2.total_lines ? t2.total_lines.toLocaleString() : '—' }}</td>
              <td>{{ t2.file_size ? formatSize(t2.file_size) : '—' }}</td>
              <td>{{ t2.stop_on_error ? '✓' : '—' }}</td>
              <td>{{ t2.single_transaction ? '✓' : '—' }}</td>
              <td>
                <span v-if="t2.status === 'running'">{{ t('import.taskRunning') }} ({{ t2.current }}/{{ t2.total || '?' }})</span>
                <span v-else-if="t2.status === 'completed'">{{ t('import.taskCompleted') }} ({{ t2.succeeded }})</span>
                <span v-else-if="t2.status === 'failed'" class="task-fail">{{ t('import.taskFailed') }} ({{ t2.failed }})</span>
                <span v-else-if="t2.status === 'cancelled'">{{ t('import.taskCancelled') }}</span>
              </td>
              <td>{{ t2.finished_at ? formatDuration(t2.duration_ms) : '—' }}</td>
              <td v-if="t2.status === 'running'"><a href="#" @click.stop.prevent="goTaskDetail(t2)">{{ t('import.detail') }}</a></td>
            </tr>
            <tr v-if="taskList.length === 0">
              <td :colspan="hasRunningTask ? 9 : 8" class="task-empty">{{ t('import.taskEmpty') }}</td>
            </tr>
          </tbody>
        </table>
      </div>
    </div>
  </div>
</template>

<style scoped>
.import-mode {
  margin-bottom: 12px;
}

.import-mode label {
  font-size: 13px;
  cursor: pointer;
}

.import-input {
  margin-bottom: 12px;
  display: flex;
  align-items: center;
  gap: 4px;
}

.file-label {
  cursor: pointer;
}

.file-btn {
  display: inline-block;
  padding: 2px 12px;
  border: 1px solid #999;
  border-radius: 3px;
  font-size: 13px;
  background: #f5f5f5;
}

.file-btn:hover {
  background: #e8e8e8;
}

.file-info {
  font-size: 13px;
  color: #666;
  margin-left: 8px;
}

.preview-section {
  margin-top: 12px;
}

.preview-header {
  font-size: 12px;
  color: #666;
  margin-bottom: 4px;
}

.preview-box {
  max-height: 500px;
  overflow: auto;
  border: 1px solid #ccc;
  border-radius: 4px;
  padding: 8px;
  font-size: 12px;
  font-family: var(--adminer-font, monospace);
  background: #fafafa;
  white-space: pre-wrap;
  word-break: break-all;
}

.execute-bar {
  margin-top: 12px;
  display: flex;
  align-items: center;
}

.stop-on-error {
  font-size: 13px;
  white-space: nowrap;
}

.tx-hint {
  margin-top: 6px;
  font-size: 12px;
  color: #888;
}

.results-section {
  margin-top: 16px;
}

.task-meta {
  margin-top: 12px;
  font-size: 12px;
  color: #666;
  display: flex;
  flex-wrap: wrap;
  gap: 16px;
}

.cancel-btn {
  padding: 2px 10px;
  border: 1px solid #c00;
  border-radius: 3px;
  font-size: 12px;
  color: #c00;
  background: #fff;
  cursor: pointer;
}

.cancel-btn:hover:not(:disabled) {
  background: #fff0f0;
}

.cancel-btn:disabled {
  opacity: 0.5;
  cursor: default;
}

.stat-ok {
  color: #060;
}

.stat-fail {
  color: #c00;
}

.task-list-section {
  margin-top: 24px;
  border-top: 1px solid #ddd;
  padding-top: 12px;
}

.task-list-title {
  font-size: 14px;
  font-weight: bold;
  margin-bottom: 8px;
}

.task-table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
}

.task-table th,
.task-table td {
  border: 1px solid #e0e0e0;
  padding: 4px 8px;
  text-align: left;
  white-space: nowrap;
}

.task-fail {
  color: #c00;
}

.task-empty {
  text-align: center;
  color: #999;
  padding: 12px;
}

.progress-section {
  margin-top: 16px;
}

.progress-text {
  font-size: 13px;
  color: #333;
  margin-bottom: 4px;
}

.progress-stats {
  color: #666;
}

.progress-bar {
  height: 8px;
  border: 1px solid #ccc;
  border-radius: 4px;
  background: #f0f0f0;
  overflow: hidden;
}

.progress-fill {
  height: 100%;
  background: #4caf50;
  transition: width 0.3s ease;
}

.error-list {
  margin-top: 8px;
}

.error-item {
  margin-bottom: 8px;
  border: 1px solid #e0e0e0;
  border-radius: 4px;
  padding: 6px 10px;
  background: #fff5f5;
}

.error-index {
  font-size: 12px;
  font-weight: bold;
  color: #c00;
  margin-bottom: 2px;
}

.error-detail {
  font-size: 12px;
  font-family: var(--adminer-font, monospace);
  color: #333;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
}
</style>
