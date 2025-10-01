<template>
    <div :class="['plan-calc-banner', bgClass]">
        <div class="content">
            <div class="left">
                <div v-if="showSpinner" class="spinner" aria-hidden="true"></div>
                <div class="text">
                    <div v-if="isLoading">loading</div>
                    <div v-else-if="state == CalculationState.Calculating">calculating</div>
                    <div v-else-if="state == CalculationState.Modified">modified</div>
                    <div v-else-if="state == CalculationState.Finished">finished</div>
                    <div v-else>{{ state }}</div>
                </div>
            </div>
            <div class="right">
                <q-btn v-if="state == CalculationState.Modified" dense flat label="Recalculate"
                    @click.prevent="recalc" />
            </div>
        </div>
    </div>
</template>

<script setup lang="ts">
import { computed } from 'vue';
import { usePlanStore } from 'src/stores/plan';
import { CalculationState } from 'src/gql/graphql';


const plan = usePlanStore();
const state = computed(() => plan.calculationState);
const isLoading = computed(() => plan.loading);
const showSpinner = computed(() => isLoading.value || state.value === CalculationState.Calculating);

async function recalc() {
    await plan.recalculate()
}

const bgClass = computed(() => {
    if (isLoading.value) return 'yellow';
    if (state.value === CalculationState.Finished) return 'green';
    if (state.value === CalculationState.Calculating || state.value === CalculationState.Modified) return 'yellow';
    return 'yellow';
});
</script>

<style scoped>
.plan-calc-banner {
    color: #222;
}

.plan-calc-banner.green {
    background: #b7f5b7;
}

.plan-calc-banner.yellow {
    background: #fff4b1;
}

.plan-calc-banner .content {
    display: flex;
    align-items: center;
    justify-content: space-between
}

.plan-calc-banner .left {
    padding: 6px 12px;
    display: flex;
    align-items: center;
    gap: 8px
}

.spinner {
    width: 18px;
    height: 18px;
    border-radius: 50%;
    border: 3px solid rgba(0, 0, 0, 0.15);
    border-top-color: rgba(0, 0, 0, 0.6);
    animation: spin 1s linear infinite;
}

@keyframes spin {
    to {
        transform: rotate(360deg);
    }
}
</style>
