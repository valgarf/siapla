<template>
    <DialogLayout :dialogLayer="dialogLayer">
        <template #toolbar>
            <div class="col"></div>
            <q-btn flat @click="toggleEdit()" :loading="resourceStore.saving" color="primary"
                :disable="resourceStore.deleting" :icon="edit ? undefined : 'edit'" class="q-ma-xs">{{ edit ? "save"
                    : null }}
            </q-btn>
            <q-btn flat @click="deleteResource()" :loading="resourceStore.deleting" color="negative" icon="delete"
                :disable="resourceStore.saving" class="q-ma-xs"></q-btn>
        </template>
        <q-card-section>
            <q-input v-if="edit" outlined placeholder="Name" class="text-h5" v-model="local_resource.name" />
            <div v-else class="text-h5">{{ local_resource.name }}</div>
        </q-card-section>

        <q-card-section>
            <DateTimeInput v-if="edit" label="Added" v-model="local_resource.added" />
            <div v-else class="row items-baseline">
                <div class="text-subtitle2 q-pr-md">Added:</div>
                <div>{{ format_datetime(local_resource.added) }}</div>
            </div>
        </q-card-section>
        <q-card-section>
            <DateTimeInput v-if="edit" label="Removed" v-model="local_resource.removed" />
            <div v-else class="row items-baseline">
                <div class="text-subtitle2 q-pr-md">Removed:</div>
                <div>{{ format_datetime(local_resource.removed) }}</div>
            </div>
        </q-card-section>

        <q-card-section>
            <div class="text-subtitle2 q-pb-sm">Holiday Calendar</div>
            <div v-if="edit" class="q-gutter-y-md">
                <q-select v-model="selectedCountry" :options="countries" option-label="name" option-value="isocode"
                    label="Country" outlined dense emit-value map-options clearable class="q-mb-md" />
                <q-select v-if="regions.length > 0" v-model="selectedRegion" :options="regions" option-label="name"
                    option-value="isocode" label="Region" outlined dense emit-value map-options clearable
                    class="q-mb-md" />
            </div>
            <div v-else class="row items-baseline">
                <div>{{ local_resource.holiday?.name || '<No holiday calendar selected>' }}</div>
            </div>
        </q-card-section>

        <q-card-section>
            <div class="text-subtitle2 q-pb-sm">Working Hours per day:</div>
            <div v-if="edit" class="row q-col-gutter-md">
                <div v-for="day in ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday']"
                    :key="day + '-edit'" class="col-12 col-sm-6">
                    <q-input :label="day" type="number" min="0" max="24" step="0.5"
                        v-model.number="local_resource.availability[day.toLowerCase().substring(0, 2) as keyof Availability]"
                        dense outlined />
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
        </q-card-section>

        <q-card-section>
            <div class="text-subtitle2 q-pb-sm">Vacations</div>
            <div v-if="edit" class="q-gutter-y-md">
                <div v-for="(vacation, index) in local_resource.vacations" :key="index+'-vacation-edit'" class="row items-center q-gutter-sm">
                    <DateTimeInput v-model="vacation.from" label="From" outlined dense class="col" />
                    <DateTimeInput v-model="vacation.until" label="Until" outlined dense class="col" />
                    <q-btn flat round color="negative" icon="delete" @click="removeVacation(index)" />
                </div>
                <q-btn @click="addVacation" icon="add" label="Add Vacation" color="primary" flat />
            </div>
            <div v-else>
                <div v-for="(vacation, index) in local_resource.vacations" :key="index+'-vacation-show'" class="q-py-xs">
                    {{ format_datetime(vacation.from) }} - {{ format_datetime(vacation.until) }}
                </div>
                <div v-if="local_resource.vacations.length == 0">No vacations scheduled</div>
            </div>
        </q-card-section>
    </DialogLayout>
</template>


<script setup lang="ts">
import { Dialog } from 'quasar'
import { ref, watch, watchEffect, computed } from 'vue';
import { type Availability, type ResourceInput, useResourceStore, defaultAvailability} from 'src/stores/resource';
import DateTimeInput from './DateTimeInput.vue';
import { format_datetime } from 'src/common/datetime'
import { useDialogStore } from 'src/stores/dialog';
import DialogLayout from './DialogLayout.vue';
import { useQuery } from '@vue/apollo-composable';
import gql from 'graphql-tag';

const resourceStore = useResourceStore();
const dialogStore = useDialogStore();

const groupedWorkingHours = computed(() => {
    if (!local_resource.value) return [];

    const days: string[] = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
    const result: Array<[string[], number]> = [];

    // Initialize with the first day
    let currentHours = local_resource.value.availability?.mo || 0;
    let currentGroup: string[] = [days[0] as string];

    // Process each day in order
    for (const day of days.slice(1)) {
        const dayKey = day.toLowerCase().substring(0, 2) as keyof Availability;
        const dayHours = local_resource.value.availability?.[dayKey] || 0;

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

const local_resource_default: ResourceInput = {
    name: "",
    timezone: Intl.DateTimeFormat().resolvedOptions().timeZone,
    added: new Date(),
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

const local_resource = ref<ResourceInput>(local_resource_default)

const edit = ref(local_resource.value.dbId == null)

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
    local_resource.value.holiday = holiday || null;
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
    dialogLayer: number;
    resource: ResourceInput;
};

const props = defineProps<Props>();

watchEffect(() => {
    // resource changed
    local_resource.value = { ...local_resource_default, ...props.resource }
    edit.value = local_resource.value.dbId == null
})

watchEffect(() => {
    if (local_resource.value.holiday) {
        selectedCountry.value = local_resource.value.holiday.country?.isocode ?? null
        selectedRegion.value = local_resource.value.holiday.region?.isocode ?? null
    }
})

watchEffect(() => {
    console.log("local_resource", local_resource.value)
})

// actions

async function toggleEdit() {
    if (edit.value) {
        await save()
        edit.value = false
    }
    else {
        edit.value = true
    }
}

function addVacation() {
    const newVacation = { 
        dbId: null,
        from: new Date(),
        until: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000)
    };
    local_resource.value.vacations.push(newVacation);
    // Also add to addedVacations for tracking
    local_resource.value.addedVacations.push({ 
        from: newVacation.from,
        until: newVacation.until 
    });
}

function removeVacation(index: number) {
    const vacation = local_resource.value.vacations[index];
    if (vacation?.dbId) {
        local_resource.value.removedVacations.push(vacation.dbId);
    } else if (vacation != null) {
        // If it's a newly added vacation that wasn't saved yet, remove it from addedVacations
        const addedIndex = local_resource.value.addedVacations.findIndex(
            v => v.from === vacation.from && v.until === vacation.until
        );
        if (addedIndex !== -1) {
            local_resource.value.addedVacations.splice(addedIndex, 1);
        }
    }
    local_resource.value.vacations.splice(index, 1);
}

async function save() {
    await resourceStore.saveResource(local_resource);
}

async function deleteResource() {
    const resourceId = local_resource.value.dbId
    if (resourceId == null) {
        dialogStore.popDialog()
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