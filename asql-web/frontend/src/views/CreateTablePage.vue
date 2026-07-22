<script setup lang="ts">
import { ref, computed, watch, onMounted, inject } from 'vue'
import { useI18n } from 'vue-i18n'
import { useRoute, useRouter } from 'vue-router'
import SearchableSelect from '../components/SearchableSelect.vue'
import { api } from '../api'
import type { Ref } from 'vue'

const route = useRoute()
const router = useRouter()

const { t } = useI18n()

const connection = computed(() => route.params.connection as string)
const database = computed(() => route.params.db as string)

const tableName = ref('')
const engine = ref('InnoDB')
const collation = ref('utf8mb4_general_ci')
const tableComment = ref('')
const saving = ref(false)

interface ColumnDef {
  name: string
  type: string
  length: string
  options: string
  nullable: boolean
  autoIncrement: boolean
  default: string
  comment: string
}

const columns = ref<ColumnDef[]>([
  { name: 'id', type: 'int', length: '11', options: 'unsigned', nullable: false, autoIncrement: true, default: '', comment: '' },
])

const aiColumnIndex = ref(0)

watch(aiColumnIndex, (idx) => {
  columns.value.forEach((c, i) => { c.autoIncrement = i === idx })
})
const showDefaultValues = ref(false)
const showComment = ref(false)
const columnOptions = computed(() => [
  { value: '', label: '-' },
  { value: 'unsigned', label: t('createTable.unsigned') },
  { value: 'zerofill', label: t('createTable.zerofill') },
  { value: 'unsigned zerofill', label: t('createTable.unsigned') + ' ' + t('createTable.zerofill') }
])

const typeGroups = ref<Array<{ category: string; types: string[] }>>([])
const collations = ref<Array<{ charset: string; collations: string[] }>>([])

const collationOptions = computed(() => {
  const opts: { value: string; label: string }[] = []
  for (const group of collations.value) {
    for (const item of group.collations) {
      opts.push({ value: item, label: item })
    }
  }
  return opts
})

const tableListRefreshKey = inject<Ref<number>>('tableListRefreshKey', ref(0))

onMounted(async () => {
  try {
    typeGroups.value = await api.columnTypes(connection.value)
    collations.value = await api.charsets(connection.value)
  } catch { /* ignore */ }
})

function addColumn(i?: number) {
  const col: ColumnDef = { name: '', type: 'varchar', length: '255', options: '', nullable: true, autoIncrement: false, default: '', comment: '' }
  if (i !== undefined) {
    columns.value.splice(i + 1, 0, col)
  } else {
    columns.value.push(col)
  }
}

function removeColumn(i: number) {
  columns.value.splice(i, 1)
  if (aiColumnIndex.value === i) {
    aiColumnIndex.value = columns.value.length > 0 ? 0 : -1
  } else if (aiColumnIndex.value > i) {
    aiColumnIndex.value--
  }
}

function moveColumn(i: number, dir: number) {
  const j = i + dir
  if (j < 0 || j >= columns.value.length) return
  const temp = columns.value[i]
  columns.value[i] = columns.value[j]
  columns.value[j] = temp
  if (aiColumnIndex.value === i) {
    aiColumnIndex.value = j
  } else if (aiColumnIndex.value === j) {
    aiColumnIndex.value = i
  }
}

async function save() {
  if (!tableName.value.trim()) {
    alert('Table name is required')
    return
  }

  const cols = columns.value.filter(c => c.name.trim())

  if (cols.length === 0) {
    alert('At least one column is required')
    return
  }

  const aiCol = cols.find(c => c.autoIncrement)

  saving.value = true
  try {
    const result = await api.createTable(connection.value, database.value, {
      table: tableName.value,
      columns: cols.map(c => ({
        name: c.name,
        type: c.type,
        length: c.length || undefined,
        options: c.options || undefined,
        nullable: c.nullable,
        auto_increment: c.autoIncrement,
        default_value: c.default || undefined,
        comment: c.comment || undefined,
      })),
      engine: engine.value,
      collation: collation.value,
      comment: tableComment.value || undefined,
    })
    tableListRefreshKey.value++
    alert(`Table \`${tableName.value}\` created`)
    router.push(`/browse/${encodeURIComponent(connection.value)}/${encodeURIComponent(database.value)}/${encodeURIComponent(tableName.value)}/structure`)
  } catch (e: any) {
    alert(`Error: ${e.message || e}`)
  }
  saving.value = false
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('createTable.title') }}</div>
    <div class="page-content">
      <table class="form-table" style="margin-bottom:12px">
        <tbody>
          <tr>
            <th>{{ $t('createTable.name') }}</th>
            <td>
              <input v-model="tableName" type="text" style="width:200px">
              <SearchableSelect v-model="engine" :options="[{value:'InnoDB',label:'InnoDB'},{value:'MyISAM',label:'MyISAM'},{value:'MEMORY',label:'MEMORY'},{value:'CSV',label:'CSV'},{value:'ARCHIVE',label:'ARCHIVE'}]" style="margin-left:4px;min-width:100px" />
               <SearchableSelect v-model="collation" :options="collationOptions" style="margin-left:4px;min-width:180px" />
              <input v-model="tableComment" type="text" style="width:150px;margin-left:4px" :placeholder="$t('createTable.tableComment')">
              <button @click="save" :disabled="saving" style="margin-left:4px">{{ $t('createTable.create') }}</button>
            </td>
          </tr>
        </tbody>
      </table>

      <table style="width:auto">
        <thead>
          <tr>
            <th>{{ $t('createTable.columnName') }}</th>
            <th>{{ $t('createTable.dataType') }}</th>
            <th>{{ $t('createTable.length') }}</th>
            <th>Options</th>
            <th>{{ $t('createTable.nullable') }}</th>
            <th>{{ $t('createTable.autoIncrement') }}</th>
            <th v-if="showDefaultValues">{{ $t('createTable.defaultValue') }}</th>
            <th v-if="showComment">{{ $t('createTable.comment') }}</th>
            <th style="width:100px"></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(col, i) in columns" :key="i">
            <td><input v-model="col.name" type="text" style="width:150px"></td>
            <td>
              <SearchableSelect v-model="col.type" :options="typeGroups.flatMap(g=>g.types.map(t=>({value:t,label:t})))" style="width:100px" />
            </td>
            <td><input v-model="col.length" type="text" style="width:60px"></td>
            <td>
              <SearchableSelect v-model="col.options" :options="columnOptions" style="width:100px" />
            </td>
            <td><input v-model="col.nullable" type="checkbox"></td>
            <td><input type="radio" name="ai" v-model="aiColumnIndex" :value="i"></td>
            <td v-if="showDefaultValues"><input v-model="col.default" type="text" style="width:100px" placeholder="Default"></td>
            <td v-if="showComment"><input v-model="col.comment" type="text" style="width:150px" placeholder="Comment"></td>
            <td>
              <button @click="addColumn(i)" :title="$t('createTable.addColumn')">+</button>
              <button @click="moveColumn(i, -1)" :disabled="i === 0" title="Move up">↑</button>
              <button @click="moveColumn(i, 1)" :disabled="i === columns.length - 1" title="Move down">↓</button>
              <button @click="removeColumn(i)" :title="$t('createTable.remove')">×</button>
            </td>
          </tr>
        </tbody>
      </table>

      <table class="form-table" style="margin-top:8px;width:auto">
        <tbody>
          <tr>
            <td colspan="2">
              <label><input v-model="showDefaultValues" type="checkbox"> Default values</label>
              <label style="margin-left:16px"><input v-model="showComment" type="checkbox"> Comment</label>
            </td>
          </tr>
        </tbody>
      </table>

      <div style="margin-top:12px">
        <button @click="save" :disabled="saving">{{ $t('createTable.create') }}</button>
      </div>

      <fieldset style="margin-top:16px;display:inline-block">
        <legend>Partition by</legend>
        <div class="fieldset-content">
          <span style="font-size:12px;color:#999">Not implemented</span>
        </div>
      </fieldset>
    </div>
  </div>
</template>
