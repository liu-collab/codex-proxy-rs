import { createApp } from 'vue'
import App from './App.vue'
import { i18n, setLocale } from './i18n'
import { readInitialLocale } from './i18n/locale'

import { authPlugin } from './plugins/auth'
import { router } from './router'
import { pinia } from './stores'
import { useThemeStore } from './stores/modules/theme'
import '@fontsource-variable/inter'
import '@fontsource-variable/jetbrains-mono'
import '@codex-proxy/ui/styles.css'

import './styles/index.css'

const app = createApp(App)

app.use(pinia)

useThemeStore(pinia).initializeTheme()

async function mountApp() {
  await setLocale(readInitialLocale())
  app.use(i18n)
  app.use(router)
  app.use(authPlugin)
  app.mount('#app')
}

void mountApp()
