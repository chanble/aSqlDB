<script setup lang="ts">
import { ref } from 'vue'
import { useI18n } from 'vue-i18n'
import SearchableSelect from '../components/SearchableSelect.vue'

const { t } = useI18n()

const props = defineProps<{
  columns: { name: string; data_type: string }[]
}>()

const emit = defineEmits<{
  (e: 'search', keyword: string, column: string, limit: number): void
}>()

const keyword = ref('')
const column = ref('')
const limit = ref(50)

function doSearch() {
  emit('search', keyword.value, column.value, limit.value)
}
</script>

<template>
  <div class="field has-addons" style="margin-bottom:12px">
    <div class="control" style="flex:1">
      <input
        v-model="keyword"
        class="input"
        type="text"
        :placeholder="$t('searchBar.search')"
        @keyup.enter="doSearch"
      >
    </div>
    <div class="control">
      <SearchableSelect v-model="column" :options="[{value:'',label:$t('searchBar.allColumns')},...columns.map(c=>({value:c.name,label:c.name}))]" style="width:auto" />
    </div>
    <div class="control">
      <input
        v-model.number="limit"
        class="input"
        type="number"
        style="width:80px"
        :placeholder="$t('searchBar.limit')"
      >
    </div>
    <div class="control">
      <button class="button is-primary" @click="doSearch">{{ $t('searchBar.select') }}</button>
    </div>
  </div>
</template>
