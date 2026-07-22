<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import SearchableSelect from '../components/SearchableSelect.vue'

const { locale } = useI18n()

function changeLocale() {
  localStorage.setItem('locale', locale.value)
}
</script>

<template>
  <div>
    <h2 class="page-title">{{ $t('settings.title') }}</h2>

    <div class="card">
      <div class="field">
        <label class="label">{{ $t('settings.language') }}</label>
        <div class="control">
          <SearchableSelect v-model="locale" :options="[{value:'ZhCn',label:$t('app.zhCn')},{value:'En',label:$t('app.en')}]" @change="changeLocale" style="width:120px" />
        </div>
      </div>
      <p class="help-text">{{ $t('settings.help') }}</p>
    </div>
  </div>
</template>

<style scoped>
.page-title {
  font-size: 150%;
  font-weight: normal;
  margin-bottom: 16px;
}

.card {
  background: #fff;
  border: 1px solid #999;
  padding: 16px;
  max-width: 480px;
}

.help-text {
  color: #666;
  font-size: 90%;
  margin-top: 4px;
}
</style>
