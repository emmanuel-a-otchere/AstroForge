<template>
  <article class="rec-card" :data-status="recommendation.status">
    <header class="rec-header">
      <span class="kind-pill font-label">{{ kindLabel(recommendation.kind) }}</span>
      <h3 class="rec-title font-display">{{ recommendation.title }}</h3>
      <span class="status-pill" :data-status="recommendation.status">
        {{ recommendation.status }}
      </span>
    </header>

    <section class="rec-body">
      <p class="rec-section-label font-label">What this changes</p>
      <p class="rec-description font-body">{{ recommendation.description }}</p>
      <p class="rec-section-label font-label">Why</p>
      <p class="rec-rationale font-body">{{ recommendation.rationale }}</p>
    </section>

    <footer v-if="recommendation.status === 'pending'" class="rec-actions">
      <button
        type="button"
        class="reject-cta font-display"
        @click="handleDecision('rejected')"
      >
        Reject
      </button>
      <button
        type="button"
        class="accept-cta font-display"
        @click="handleDecision('accepted')"
      >
        Accept
      </button>
    </footer>
    <footer v-else class="rec-decided">
      <p class="decided-message font-body">
        {{ recommendation.status === "accepted" ? "Accepted — applied as a new version." : "Rejected — no changes made." }}
      </p>
    </footer>
  </article>
</template>

<script setup>
import { versionStore } from "@/state/versions";

const props = defineProps({
  recommendation: { type: Object, required: true },
});

function handleDecision(status) {
  versionStore.applyRecommendationStatus(props.recommendation.id, status);
}

function kindLabel(kind) {
  switch (kind) {
    case "tone_adjustment": return "Tone";
    case "star_reduction": return "Stars";
    case "color_balance": return "Color";
    case "noise_reduction": return "Noise";
    case "stretch_recommendation": return "Stretch";
    default: return kind;
  }
}
</script>

<style scoped>
.rec-card {
  background: var(--surface-container-low);
  border: 1px solid var(--outline-variant);
  border-radius: var(--radius-md);
  padding: var(--sp-lg);
  display: flex;
  flex-direction: column;
  gap: var(--sp-sm);
}

.rec-header {
  display: flex;
  align-items: center;
  gap: var(--sp-sm);
}

.kind-pill {
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--tertiary-container);
  color: var(--on-tertiary-container);
}

.rec-title {
  margin: 0;
  font-size: 1.05rem;
  flex: 1 1 auto;
}

.status-pill {
  font-size: 0.7rem;
  padding: 2px 8px;
  border-radius: var(--radius-full);
  background: var(--surface-container-high);
  color: var(--on-surface-variant);
}

.rec-section-label {
  margin: var(--sp-xs) 0 0;
  font-size: 0.7rem;
  color: var(--on-surface-variant);
  text-transform: uppercase;
}

.rec-description,
.rec-rationale {
  margin: 2px 0 0;
  font-size: 0.88rem;
}

.rec-actions {
  display: flex;
  justify-content: flex-end;
  gap: var(--sp-sm);
}

.accept-cta {
  background: var(--primary);
  color: var(--on-primary);
  border: none;
  padding: var(--sp-xs) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}

.reject-cta {
  background: transparent;
  color: var(--on-surface-variant);
  border: 1px solid var(--outline-variant);
  padding: var(--sp-xs) var(--sp-lg);
  border-radius: var(--radius-md);
  cursor: pointer;
  font-weight: 600;
}

.decided-message {
  margin: 0;
  font-size: 0.85rem;
  color: var(--on-surface-variant);
  text-align: right;
}
</style>
