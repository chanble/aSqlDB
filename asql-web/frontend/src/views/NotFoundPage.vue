<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { useRouter } from 'vue-router'
const { t } = useI18n()
const router = useRouter()
</script>

<template>
  <div style="text-align:center;padding:60px 0">
    <h2 class="page-title">{{ $t('notFound.title') }}</h2>
    <p>{{ $t('notFound.message') }}</p>
    <button class="button is-primary" @click="router.push('/')">{{ $t('notFound.goHome') }}</button>
  </div>
</template>

<style scoped>
.page-title {
  font-size: 300%;
  font-weight: normal;
  margin-bottom: 8px;
}
</style>
