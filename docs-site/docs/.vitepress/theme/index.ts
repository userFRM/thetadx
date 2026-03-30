import DefaultTheme from 'vitepress/theme'
import type { Theme } from 'vitepress'
import LanguageSelector from './components/LanguageSelector.vue'
import TierBadge from './components/TierBadge.vue'
import './style.css'

export default {
  extends: DefaultTheme,
  enhanceApp({ app }) {
    app.component('LanguageSelector', LanguageSelector)
    app.component('TierBadge', TierBadge)
  },
} satisfies Theme
