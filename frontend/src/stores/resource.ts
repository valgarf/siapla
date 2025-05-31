import { graphql } from 'src/gql';
import { defineStore, acceptHMRUpdate } from 'pinia';
import { useMutation, useQuery } from '@vue/apollo-composable';
import { Weekday } from 'src/gql/graphql';
import type { ResourceSaveInput, ResourcesQuery } from 'src/gql/graphql';
import { computed, type Ref } from 'vue';
import { ResourceDialogData, useDialogStore } from './dialog';

export interface Availability {
  mo: number;
  tu: number;
  we: number;
  th: number;
  fr: number;
  sa: number;
  su: number;
}
export interface Holiday {
  dbId: number;
  name: string;
  country: {
    name: string;
    isocode: string;
  } | null;
  region: {
    name: string;
    isocode: string;
  } | null;
}

export interface Vacation {
  dbId: number | null;
  from: Date;
  until: Date;
}

export interface Resource {
  dbId: number;
  name: string;
  timezone: string;
  added: Date;
  removed: Date | null;
  holiday?: Holiday | null;
  availability: Availability | null;
  vacations: Vacation[];
}

export interface ResourceInput extends Partial<Resource> {
  name: string;
  timezone: string;
  added: Date;
  availability: Availability;
  addedVacations: Array<{ from: Date; until: Date }>;
  removedVacations: number[];
  vacations: Vacation[];
}

const RESOURCE_QUERY = graphql(`
  query resources {
    resources {
      dbId
      name
      timezone
      added
      removed
      vacation {
        dbId
        from
        until
      }
      holiday {
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
      availability {
        weekday
        duration
      }
    }
  }
`);

const RESOURCE_SAVE_MUTATION = graphql(`
  mutation resource_save($resource: ResourceSaveInput!) {
    resourceSave(resource: $resource) {
      dbId
    }
  }
`);

const RESOURCE_DELETE_MUTATION = graphql(`
  mutation resource_delete($resourceId: Int!) {
    resourceDelete(resourceId: $resourceId)
  }
`);

// Maps Weekday enum values to their short codes
type WeekdayMap = Record<Weekday, keyof Availability>;
const weekdayMap: WeekdayMap = {
  [Weekday.Monday]: 'mo',
  [Weekday.Tuesday]: 'tu',
  [Weekday.Wednesday]: 'we',
  [Weekday.Thursday]: 'th',
  [Weekday.Friday]: 'fr',
  [Weekday.Saturday]: 'sa',
  [Weekday.Sunday]: 'su'
};

// Create reverse mapping from short codes to Weekday enum
const reverseWeekdayMap = Object.entries(weekdayMap).reduce((acc, [weekday, shortCode]) => {
  acc[shortCode] = weekday as Weekday;
  return acc;
}, {} as Record<keyof Availability, Weekday>);

export const defaultAvailability: Availability = {
  mo: 8, // default 8 hours Monday-Friday
  tu: 8,
  we: 8,
  th: 8,
  fr: 8,
  sa: 0, // default 0 hours Saturday and Sunday
  su: 0
};

function convertQueryResult(query: ResourcesQuery) {
  const resources: Map<number, Resource> = new Map(
    query.resources.map((r) => {
      // Override with values from the backend if they exist
      const availability = r.availability?.reduce((acc: Availability, item) => {
        const weekday = weekdayMap[item.weekday];
        if (weekday) {
          acc[weekday] = item.duration / 3600; // Convert seconds to hours
        }
        return acc;
      }, { ...defaultAvailability }) || defaultAvailability; // Fallback to defaults if availability is null/undefined
      
      return [
        r.dbId,
        {
          dbId: r.dbId,
          name: r.name,
          timezone: r.timezone,
          added: new Date(r.added),
          removed: r.removed == null ? null : new Date(r.removed),
          holidayId: r.holiday?.dbId ?? null,
          holidayName: r.holiday?.name ?? null,
          holiday: r.holiday ? {
            dbId: r.holiday.dbId,
            name: r.holiday.name,
            country: r.holiday.country ?? null,
            region: r.holiday.region ?? null
          } : null,
          availability,
          vacations: r.vacation ? r.vacation.map(v => ({
            dbId: v.dbId,
            from: new Date(v.from),
            until:new Date( v.until)
          })) : []
        },
      ];
    }),
  );
  return resources;
}

function resourceToObj(resource: Ref<ResourceInput>): ResourceSaveInput {
  const result: ResourceSaveInput = {
    dbId: resource.value.dbId ?? null,
    name: resource.value.name,
    timezone: resource.value.timezone,
    added: resource.value.added.toISOString(),
    removed: resource.value.removed?.toISOString(),
    holidayId: resource.value.holiday?.dbId ?? null,
    availability: Object.entries(resource.value.availability).map(([key, value]) => {
      const weekday = reverseWeekdayMap[key as keyof Availability];
      if (!weekday) {
        throw new Error(`Invalid weekday key: ${key}`);
      }
      return {
        weekday,
        duration: value * 3600
      };
    }),
    addedVacations: (resource.value.addedVacations || []).map(v => ({
      from: new Date(v.from).toISOString(),
      until: new Date(v.until).toISOString()
    })),
    removedVacations: resource.value.removedVacations || [],
  };
  console.log("Storing resource:", result);
  return result;
}

// actual store
export const useResourceStore = defineStore('resourceStore', () => {
  const queryGetAll = useQuery(RESOURCE_QUERY);
  const mutSaveResource = useMutation(RESOURCE_SAVE_MUTATION);
  const mutDeleteResource = useMutation(RESOURCE_DELETE_MUTATION);

  const apollo_objs = [queryGetAll, mutSaveResource, mutDeleteResource];
  const resource_map = computed(() => {
    if (queryGetAll.result.value == null) {
      return null;
    } else {
      return convertQueryResult(queryGetAll.result.value);
    }
  });

  async function saveResource(resource: Ref<ResourceInput>) {
    const dialog = useDialogStore();
    const resp = await mutSaveResource.mutate({ resource: resourceToObj(resource) });
    const dbId = resp?.data?.resourceSave.dbId;
    if (dbId != null) {
      if (resource.value.dbId == null) {
        // a little hacky
        // TODO: necessary?
        resource.value.dbId = dbId;
        await queryGetAll.refetch();
        // TODO: generic error handling?
        dialog.replaceDialog(new ResourceDialogData(dbId));
      } else {
        resource.value.dbId = dbId;
        await queryGetAll.refetch();
        // TODO: generic error handling?
      }
    }
  }

  async function deleteResource(resourceId: number, pop: boolean = true) {
    const dialog = useDialogStore();
    const resp = await mutDeleteResource.mutate({ resourceId: resourceId });
    const result = resp?.data?.resourceDelete;
    if (result) {
      // TODO: a 'filter' that removes all corresponding dialogs would be better
      if (
        pop &&
        dialog.activeDialog instanceof ResourceDialogData &&
        dialog.activeDialog.resourceId == resourceId
      ) {
        dialog.popDialog();
      }
      await queryGetAll.refetch();
    }
    return result;
    // TODO: generic error handling?
  }
  const resources = computed(() => Array.from(resource_map.value?.values() || []));

  return {
    gql: {
      queryGetAll,
      mutSaveResource,
    },
    loading: queryGetAll.loading,
    saving: mutSaveResource.loading,
    deleting: mutDeleteResource.loading,
    resources,
    // TODO: generic GQL error messages as notifications?
    apolloErrors: computed(() => apollo_objs.map((obj) => obj.error).filter((err) => err != null)),
    resource: (dbId: number): Resource | undefined => {
      return resource_map.value?.get(dbId);
    },
    saveResource,
    deleteResource,
  };
});

if (import.meta.hot) {
  import.meta.hot.accept(acceptHMRUpdate(useResourceStore, import.meta.hot));
}
