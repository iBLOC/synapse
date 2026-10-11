// Theme extension: default theme + a doc-after slot that renders the
// "Copy page as Markdown" actions on every docs page.
import { h } from 'vue'
import DefaultTheme from 'vitepress/theme'
import DocActions from './components/DocActions.vue'
import './custom.css'

export default {
  extends: DefaultTheme,
  Layout() {
    return h(DefaultTheme.Layout, null, {
      'doc-after': () => h(DocActions),
    })
  },
}
