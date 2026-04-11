import { createApp } from 'vue'
import { Quasar, Notify, Dialog, Dark, LocalStorage } from 'quasar'
import router from './router'
import pinia from './stores'

import '@quasar/extras/material-icons/material-icons.css'
import '@quasar/extras/material-icons-outlined/material-icons-outlined.css'
import 'quasar/src/css/index.sass'
import './css/app.sass'

import App from './App.vue'

// Restore theme preference
const savedDark = LocalStorage.getItem<boolean>('darkMode')
Dark.set(savedDark !== null ? savedDark : true)

const app = createApp(App)
app.use(Quasar, {
  plugins: { Notify, Dialog },
  config: {
    dark: 'auto',
    notify: { position: 'bottom-right' }
  }
})
app.use(router)
app.use(pinia)
app.mount('#app')
