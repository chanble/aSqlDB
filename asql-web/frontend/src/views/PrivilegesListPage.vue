<script setup lang="ts">
import { useI18n } from 'vue-i18n'
import { ref, computed, onMounted } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { api } from '../api'

const { t } = useI18n()
const route = useRoute()
const router = useRouter()

const connection = computed(() => route.params.connection as string)
const users = ref<{ user: string; host: string }[]>([])
const loading = ref(true)

onMounted(async () => {
  await loadUsers()
  loading.value = false
})

async function loadUsers() {
  if (!connection.value) return
  try {
    const result = await api.listUsers(connection.value)
    users.value = result.data.map((r: any) => ({
      user: r.user || '',
      host: r.host || '',
    }))
  } catch { /* ignore */ }
}

function goCreateUser() {
  router.push(`/create-user/${encodeURIComponent(connection.value)}`)
}

function goEditUser(user: string, host: string) {
  router.push(`/edit-user/${encodeURIComponent(connection.value)}?user=${encodeURIComponent(user)}&host=${encodeURIComponent(host)}`)
}
</script>

<template>
  <div>
    <div class="page-header">{{ $t('privileges.title') }}</div>
    <div class="page-content">
      <div style="margin-bottom:12px">
        <a href="#" @click.prevent="goCreateUser">{{ $t('privileges.createUser') }}</a>
      </div>

      <div v-if="loading" style="padding:20px;text-align:center;color:#999">{{ $t('common.loading') }}</div>

      <table v-else style="width:auto">
        <thead>
          <tr>
            <th>{{ $t('privileges.username') }}</th>
            <th>{{ $t('privileges.host') }}</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="(u, i) in users" :key="i">
            <td>{{ u.user }}</td>
            <td>{{ u.host }}</td>
            <td><a href="#" @click.prevent="goEditUser(u.user, u.host)">{{ $t('privileges.edit') }}</a></td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>
