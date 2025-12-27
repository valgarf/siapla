import { reactive } from "vue";
export interface SortOption { key: string; label: string; asc: boolean }

export const taskSortOptions = reactive<SortOption[]>([
    { key: 'isRequirement', label: 'Is Requirement', asc: false },
    { key: 'isMilestone', label: 'Is Milestone', asc: true },
    { key: 'start', label: 'Start', asc: true },
    { key: 'end', label: 'End', asc: true },
    { key: 'isBooked', label: 'Is Booked', asc: false },
    { key: 'effort', label: 'Effort', asc: false },
    { key: 'name', label: 'Name', asc: true },
    { key: 'isGroup', label: 'Is Group', asc: true },
]);

export const resourceSortOptions = reactive<SortOption[]>([
    { key: 'name', label: 'Name', asc: true },
    { key: 'added', label: 'Added', asc: true },
    { key: 'removed', label: 'Removed', asc: true },
    { key: 'earliestStart', label: 'Earliest Task Start', asc: true },
    { key: 'lastEnd', label: 'Last Task End', asc: true },
    { key: 'totalHours', label: 'Total Working hours', asc: false }
]);