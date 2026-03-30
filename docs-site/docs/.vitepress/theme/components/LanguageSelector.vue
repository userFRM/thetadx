<script setup lang="ts">
import { ref, onMounted, watch } from 'vue'

interface Language {
  id: string
  name: string
}

const languages: Language[] = [
  { id: 'rust', name: 'Rust' },
  { id: 'python', name: 'Python' },
  { id: 'go', name: 'Go' },
  { id: 'cpp', name: 'C++' },
]

const STORAGE_KEY = 'thetadatadx-preferred-lang'

const activeLanguage = ref('python')

onMounted(() => {
  const stored = localStorage.getItem(STORAGE_KEY)
  if (stored && languages.some((l) => l.id === stored)) {
    activeLanguage.value = stored
  }
  // Dispatch initial event so any listeners can pick up the default
  dispatchLanguageEvent(activeLanguage.value)
})

watch(activeLanguage, (newLang) => {
  localStorage.setItem(STORAGE_KEY, newLang)
  dispatchLanguageEvent(newLang)
})

function dispatchLanguageEvent(lang: string) {
  if (typeof window !== 'undefined') {
    window.dispatchEvent(
      new CustomEvent('thetadatadx-lang-change', { detail: { language: lang } })
    )
    // Also set a data attribute on <html> so CSS-only selectors can react
    document.documentElement.setAttribute('data-lang', lang)
  }
}

function selectLanguage(id: string) {
  activeLanguage.value = id
}
</script>

<template>
  <div class="language-selector">
    <span class="language-selector-label">SDK Language:</span>
    <div class="language-pills">
      <button
        v-for="lang in languages"
        :key="lang.id"
        :class="['language-pill', { active: activeLanguage === lang.id }]"
        @click="selectLanguage(lang.id)"
        :aria-pressed="activeLanguage === lang.id"
        :title="`Show ${lang.name} examples`"
      >
        <img
          class="language-icon"
          :src="`/ThetaDataDx/icons/${lang.id === 'cpp' ? 'cplusplus' : lang.id}.svg`"
          :alt="lang.name"
        />
        <span class="language-name">{{ lang.name }}</span>
      </button>
    </div>
  </div>
</template>

<style scoped>
.language-selector {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 12px 0;
  margin-bottom: 16px;
  border-bottom: 1px solid var(--vp-c-divider);
}

.language-selector-label {
  font-size: 13px;
  font-weight: 600;
  color: var(--vp-c-text-2);
  white-space: nowrap;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.language-pills {
  display: flex;
  gap: 8px;
  flex-wrap: wrap;
}

.language-pill {
  display: inline-flex;
  align-items: center;
  gap: 6px;
  padding: 6px 14px;
  border-radius: 20px;
  border: 1px solid var(--vp-c-divider);
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-2);
  font-size: 13px;
  font-weight: 500;
  cursor: pointer;
  transition: all 0.2s ease;
  user-select: none;
}

.language-pill:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
  background: var(--vp-c-brand-soft);
}

.language-pill.active {
  border-color: var(--vp-c-brand-1);
  background: var(--vp-c-brand-1);
  color: #fff;
  font-weight: 600;
  box-shadow: 0 1px 4px rgba(59, 130, 246, 0.3);
}

.language-icon {
  width: 16px;
  height: 16px;
  flex-shrink: 0;
}

.language-name {
  line-height: 1;
}
</style>
