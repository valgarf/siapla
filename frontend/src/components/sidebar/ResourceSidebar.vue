<template>
    <SidebarLayout>
        <template #toolbar>
            <div class="col"></div>
            <q-btn flat @click="toggleEdit()" :loading="resourceStore.saving" color="primary"
                :disable="resourceStore.deleting" :icon="edit ? 'save' : 'edit'" class="q-ma-xs"
                :class="{ shake: sidebarStore.shakeButtons }" />
            <q-btn v-if="edit && localResource.dbId != null" flat round icon="cancel" aria-label="Cancel"
                class="q-ma-xs" @click="cancelEdit" :class="{ shake: sidebarStore.shakeButtons }" />
            <q-btn flat @click="deleteResource()" :loading="resourceStore.deleting" color="negative" icon="delete"
                :disable="resourceStore.saving" class="q-ma-xs" :class="{ shake: sidebarStore.shakeButtons }"></q-btn>
        </template>
        <q-banner v-if="saveError" dense class="text-white bg-red">{{ saveError }}</q-banner>

        <div class="sidebar-content">

            <section class="sidebar-section responsive-row">
                <q-input v-if="edit" outlined placeholder="Name" class="text-h5 responsive-field"
                    v-model="localResource.name" />
                <div v-else class="text-h5">{{ localResource.name }}</div>
            </section>

            <section class="sidebar-section responsive-row">
                <DateTimeInput v-if="edit" label="Added" class="responsive-field" v-model="localResource.added" />
                <div v-else class="row items-baseline responsive-field">
                    <div class="text-subtitle2 q-pr-md">Added:</div>
                    <div>{{ formatDatetime(localResource.added) }}</div>
                </div>
                <DateTimeInput v-if="edit" label="Removed" class="responsive-field" v-model="localResource.removed" />
                <div v-else class="row items-baseline responsive-field">
                    <div class="text-subtitle2 q-pr-md">Removed:</div>
                    <div>{{ formatDatetime(localResource.removed) }}</div>
                </div>
            </section>

            <section class="sidebar-section">
                <div class="text-subtitle2 q-pb-sm">Holiday Calendar</div>
                <div v-if="edit" class="q-gutter-y-md responsive-row">
                    <q-select v-model="selectedCountry" :options="countries" option-label="name" option-value="isocode"
                        label="Country" outlined dense emit-value map-options clearable
                        class="q-mb-md responsive-field" />
                    <q-select v-if="regions.length > 0" v-model="selectedRegion" :options="regions" option-label="name"
                        option-value="isocode" label="Region" outlined dense emit-value map-options clearable
                        class="q-mb-md responsive-field" />
                </div>
                <div v-else class="row items-baseline">
                    <div>{{ localResource.holiday?.name || '<No holiday calendar selected>' }}</div>
                </div>
            </section>

            <section class="sidebar-section">
                <div class="text-subtitle2 q-pb-sm">Working Hours per day:</div>
                <div v-if="edit" class="working-hours-grid">
                    <div v-for="day in ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday']"
                        :key="day + '-edit'" class="working-day">
                        <q-input :label="day" type="number" min="0" max="24" step="0.5"
                            v-model.number="localResource.availability[day.toLowerCase().substring(0, 2) as keyof Availability]"
                            dense outlined class="responsive-field" />
                    </div>
                </div>
                <div v-else>
                    <div v-for="([days, hours], index) in groupedWorkingHours" :key="index"
                        class="row items-center q-mb-xs">
                        <div class="col-4 text-body2">
                            {{ formatDayRange(days) }}
                        </div>
                        <div class="col-2">{{ hours }}h</div>
                    </div>
                </div>
            </section>

            <section class="sidebar-section">
                <div class="text-subtitle2 q-pb-sm">Vacations</div>
                <div v-if="edit" class="q-gutter-y-md">
                    <div v-for="(vacation, index) in localResource.vacations" :key="index + '-vacation-edit'"
                        class="row items-center q-gutter-sm">
                        <DateTimeInput v-model="vacation.from" label="From" outlined class="col responsive-field" />
                        <DateTimeInput v-model="vacation.until" label="Until" outlined class="col responsive-field" />
                        <q-btn flat round color="negative" icon="delete" @click="removeVacation(index)" />
                    </div>
                    <q-btn @click="addVacation" icon="add" label="Add Vacation" color="primary" flat />
                </div>
                <div v-else>
                    <div v-for="(vacation, index) in localResource.vacations" :key="index + '-vacation-show'"
                        class="q-py-xs">
                        {{ formatDatetime(vacation.from) }} - {{ formatDatetime(vacation.until) }}
                    </div>
                    <div v-if="localResource.vacations.length == 0">No vacations scheduled</div>
                </div>
            </section>
        </div>
    </SidebarLayout>
</template>


<script setup lang="ts">
import { useQuery } from '@vue/apollo-composable';
import gql from 'graphql-tag';
import { Dialog } from 'quasar';
import { formatDatetime } from 'src/common/datetime';
import { useSidebarStore } from 'src/stores/sidebar';
import { type Availability, defaultAvailability, type ResourceInput, useResourceStore, type Vacation } from 'src/stores/resource';
import { computed, ref, watch, watchEffect } from 'vue';
import DateTimeInput from '../forms/DateTimeInput.vue';
import SidebarLayout from './SidebarLayout.vue';

const resourceStore = useResourceStore();
const sidebarStore = useSidebarStore();

const groupedWorkingHours = computed(() => {
    if (!localResource.value) return [];

    const days: string[] = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
    const result: Array<[string[], number]> = [];

    // Initialize with the first day
    let currentHours = localResource.value.availability?.mo || 0;
    let currentGroup: string[] = [days[0] as string];

    // Process each day in order
    for (const day of days.slice(1)) {
        const dayKey = day.toLowerCase().substring(0, 2) as keyof Availability;
        const dayHours = localResource.value.availability?.[dayKey] || 0;

        // If hours match the current group, add to group
        if (Math.abs(dayHours - currentHours) < 0.01) {
            currentGroup.push(day);
        } else {
            // Add the current group to the result
            result.push([[...currentGroup], currentHours]);

            // Start a new group
            currentHours = dayHours;
            currentGroup = [day];
        }
    }

    if (currentGroup.length > 0) {
        result.push([currentGroup, currentHours]);
    }

    return result;
})

const currentDate = new Date()
currentDate.setHours(0, 0, 0, 0);
const localResourceDefault: ResourceInput = {
    name: "",
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    added: currentDate,
    availability: { ...defaultAvailability },
    removed: null,
    holiday: null,
    vacations: [],
    addedVacations: [],
    removedVacations: []
};

function formatDayRange(days: string[]): string {
    if (!days || days.length === 0) return '';
    if (days.length === 1) return days[0] || '';
    const first = days[0] || '';
    const last = days[days.length - 1] || '';
    return `${first} - ${last}`;
}

const localResource = ref<ResourceInput>(localResourceDefault)

const edit = ref(localResource.value.dbId == null)

// Holiday selection state
const selectedCountry = ref<string | null>(null);
const selectedRegion = ref<string | null>(null);

const { result: countriesResult } = useQuery(gql`
  query GetCountries {
    countries {
      isocode
      name
    }
  }
`)

const countries = computed(() => {
    const countriesList = countriesResult.value?.countries || [];
    return [...countriesList].sort((a: { name: string }, b: { name: string }) => a.name.localeCompare(b.name));
});

const regionsVariables = computed(() => {
    return { isocode: selectedCountry.value }
})

const { result: regionsResult, loading: regionsLoading, error: regionsError, onResult: onRegionsResult } = useQuery(gql`
    query GetRegions($isocode: String!) {
      country(isocode: $isocode) {
        regions {
          name
          isocode
        }
      }
    }
  `, regionsVariables,
    { enabled: computed(() => selectedCountry.value != null) }
)

const regions = computed(() => {
    const regionsList = selectedCountry.value != null ? regionsResult.value?.country?.regions || [] : [];
    return [...regionsList].sort((a: { name: string }, b: { name: string }) => a.name.localeCompare(b.name));
});

// Compute the current ISO code based on selected region or country
const currentIsoCode = computed(() => {
    if (selectedRegion.value) {
        return selectedRegion.value
    }
    if (!regionsError.value && !regionsLoading.value && regions.value.length == 0) {
        return selectedCountry.value
    }
    return null
})

// Query for holiday information
const { result: holidayResult } = useQuery(gql`
  query GetHoliday($isocode: String!) {
    getFromOpenHolidays(isocode: $isocode) {
      dbId
      name
      country {
        name
        isocode
      }
      region {
        name
        isocode
      }
    }
  }
`,
    computed(() => { return { isocode: currentIsoCode.value } }),
    { enabled: computed(() => !!currentIsoCode.value) }
)

// Compute the holiday ID from the query result
watch(() => holidayResult.value?.getFromOpenHolidays, (holiday) => {
    localResource.value.holiday = holiday || null;
})

onRegionsResult((result) => {
    if (selectedRegion.value != null && !result.loading && !result.error && !result.data?.country?.regions.some((r: { isocode: string }) => r.isocode == selectedRegion.value)) {
        // console.log("resetting selectedRegion.value", selectedRegion.value)
        // console.log(selectedCountry.value, result)
        selectedRegion.value = null;
    }
})
// holiday logic end

interface Props {
    resource: ResourceInput;
};

const props = defineProps<Props>();
let originalVacations: Vacation[] = [];
watchEffect(() => {
    // resource changed
    localResource.value = { ...localResourceDefault, ...props.resource }
    edit.value = localResource.value.dbId == null
    originalVacations = [...props.resource.vacations?.map(v => ({ ...v })) || []];
    console.assert(originalVacations.every(v => v.dbId != null), "assertion failed: all vacations should have a dbId")

})

watchEffect(() => {
    if (localResource.value.holiday) {
        selectedCountry.value = localResource.value.holiday.country?.isocode ?? null
        selectedRegion.value = localResource.value.holiday.region?.isocode ?? null
    }
})

// actions

const saveError = ref<string | null>(null)

async function toggleEdit() {
    if (edit.value) {
        const err = await save()
        saveError.value = err
        if (!err) {
            edit.value = false
            sidebarStore.setEditing(false)
        }
    }
    else {
        saveError.value = null
        edit.value = true
        sidebarStore.setEditing(true)
    }
}

function cancelEdit() {
    // local reset
    localResource.value = { ...localResourceDefault, ...props.resource };
    saveError.value = null;
    edit.value = false;
    // always allow cancel at store level (removes new items)
    sidebarStore.cancelEdit();
    sidebarStore.setEditing(false)
}

function addVacation() {
    const now = new Date()
    now.setHours(0)
    now.setMinutes(0)
    now.setSeconds(0)
    now.setMilliseconds(0)
    const newVacation = {
        dbId: null,
        from: now,
        until: new Date(now.getTime() + 7 * 24 * 60 * 60 * 1000)
    };
    localResource.value.vacations.push(newVacation);
}

function removeVacation(index: number) {
    localResource.value.vacations.splice(index, 1);
}

async function save(): Promise<string | null> {
    let addedVacations = localResource.value.vacations.filter(v => !originalVacations.some(v2 => v2.dbId == v.dbId)).map(v => ({ from: v.from, until: v.until }));
    let removedVacations: number[] = originalVacations.filter(v => !localResource.value.vacations.some(v2 => v2.dbId == v.dbId && v.dbId != null)).map(v => v.dbId as number);
    const modifiedVacations = localResource.value.vacations.filter(v => originalVacations.some(v2 => v2.dbId == v.dbId && v.dbId != null && (v2.from != v.from || v2.until != v.until)));
    addedVacations = [...addedVacations, ...modifiedVacations.map(v => ({ from: v.from, until: v.until }))];
    removedVacations = [...removedVacations, ...modifiedVacations.map(v => v.dbId as number)];
    localResource.value.addedVacations = addedVacations
    localResource.value.removedVacations = removedVacations
    localResource.value.vacations = [];
    const err = await resourceStore.saveResource(localResource);
    // Only clear originalVacations on success
    if (!err) originalVacations = [...localResource.value.vacations.map(v => ({ ...v }))];
    return err
}

async function deleteResource() {
    const resourceId = localResource.value.dbId
    if (resourceId == null) {
        cancelEdit()
        return
    }
    const dialogResolved = new Promise((resolve, reject) => {
        Dialog.create({
            title: 'Delete?',
            message: 'Would you really like to delete the resource?',
            cancel: true,
            persistent: true
        }).onOk(resolve).onCancel(reject).onDismiss(reject)
    })
    try {
        await dialogResolved
    } catch {
        return
    }
    await resourceStore.deleteResource(resourceId, true);
}

</script>

<style scoped>
.sidebar-content {
    display: flex;
    flex-flow: column nowrap;
    gap: 0px;
    align-items: stretch;
}

.sidebar-section {
    padding: 12px 12px;
    border-bottom: 1px solid #f0f0f0;
}

.responsive-field {
    width: 100%;
    max-width: 520px;
    box-sizing: border-box;
}

.responsive-row {
    display: flex;
    gap: 12px;
    align-items: center;
    flex-wrap: wrap;
}

.working-hours-grid {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
    align-items: center;
}

.working-day {
    flex: 0 0 96px;
}

.bookings-row {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    align-items: flex-start;
}

/* shake animation for blocked save/cancel */
@keyframes shake {

    10%,
    90% {
        transform: translate3d(-1px, 0, 0);
    }

    20%,
    80% {
        transform: translate3d(2px, 0, 0);
    }

    30%,
    50%,
    70% {
        transform: translate3d(-4px, 0, 0);
    }

    40%,
    60% {
        transform: translate3d(4px, 0, 0);
    }
}

.shake {
    animation: shake 0.6s;
}
</style>
