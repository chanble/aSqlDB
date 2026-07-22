import { createI18n } from 'vue-i18n'
import en from '../locales/en.json'
import zh from '../locales/zh.json'

const savedLocale = localStorage.getItem('locale') || (
  navigator.language.startsWith('zh') ? 'ZhCn' : 'En'
)

export const i18n = createI18n({
  legacy: false,
  locale: savedLocale,
  fallbackLocale: 'En',
  messages: {
    En: en,
    ZhCn: zh,
  },
})
