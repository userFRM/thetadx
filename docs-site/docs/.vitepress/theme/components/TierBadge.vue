<script setup lang="ts">
const props = defineProps<{
  tier: 'free' | 'value' | 'standard' | 'professional'
}>()

const tiers = ['free', 'value', 'standard', 'professional'] as const

const tierLabels: Record<string, string> = {
  free: 'Free',
  value: 'Value',
  standard: 'Standard',
  professional: 'Pro',
}

function isActive(t: string): boolean {
  const minIndex = tiers.indexOf(props.tier as any)
  const thisIndex = tiers.indexOf(t as any)
  return thisIndex >= minIndex
}
</script>

<template>
  <div class="tier-badges">
    <span
      v-for="t in tiers"
      :key="t"
      :class="['tier-badge', `tier-${t}`, { active: isActive(t), inactive: !isActive(t) }]"
    >{{ tierLabels[t] }}</span>
  </div>
</template>

<style scoped>
.tier-badges {
  display: inline-flex;
  gap: 0;
  margin: 8px 0 16px;
  border-radius: 6px;
  overflow: hidden;
  border: 1px solid var(--vp-c-divider);
}

.tier-badge {
  padding: 4px 12px;
  font-size: 12px;
  font-weight: 600;
  text-transform: uppercase;
  letter-spacing: 0.04em;
  line-height: 1;
  transition: all 0.15s ease;
}

.tier-badge.active.tier-free {
  background: #059669;
  color: #fff;
}
.tier-badge.active.tier-value {
  background: #2563eb;
  color: #fff;
}
.tier-badge.active.tier-standard {
  background: #7c3aed;
  color: #fff;
}
.tier-badge.active.tier-professional {
  background: #dc2626;
  color: #fff;
}

.tier-badge.inactive {
  background: var(--vp-c-bg-soft);
  color: var(--vp-c-text-3);
  opacity: 0.4;
}
</style>
