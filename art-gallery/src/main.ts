import { createApp } from 'vue'
import { Quasar, Notify, Dialog, Dark, LocalStorage } from 'quasar'
import router from './router'
import pinia from './stores'

import '@quasar/extras/material-icons/material-icons.css'
import '@quasar/extras/material-icons-outlined/material-icons-outlined.css'
import 'quasar/src/css/index.sass'
import './css/app.sass'

import App from './App.vue'

const app = createApp(App)
app.use(Quasar, {
  plugins: { Notify, Dialog },
  config: {
    notify: { position: 'bottom-right' }
  }
})
app.use(router)
app.use(pinia)
app.mount('#app')

// Restore theme preference after Quasar init
const savedTheme = LocalStorage.getItem<'auto' | 'light' | 'dark'>('themeMode') || 'auto'
const prefersDarkMedia = window.matchMedia('(prefers-color-scheme: dark)')
if (savedTheme === 'auto') {
  Dark.set(prefersDarkMedia.matches)
} else {
  Dark.set(savedTheme === 'dark')
}

// 系统深浅色偏好可能随时变化：auto 模式下跟随系统实时切换
prefersDarkMedia.addEventListener('change', (e) => {
  const mode = LocalStorage.getItem<'auto' | 'light' | 'dark'>('themeMode') || 'auto'
  if (mode === 'auto') Dark.set(e.matches)
})
