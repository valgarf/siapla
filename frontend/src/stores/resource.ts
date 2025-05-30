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
export interface Resource {
  dbId: number;
  name: string;
  timezone: string;
  added: Date;
  removed: Date | null;
  holidayId: number | null;
  availability: Availability | null;
}

export interface ResourceInput extends Partial<Resource> {
  name: string;
  timezone: string;
  added: Date;
  availability: Availability;
}

const RESOURCE_QUERY = graphql(`
  query resources {
    resources {
      dbId
      name
      timezone
      added
      removed
      holiday {
        dbId
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

function convertQueryResult(query: ResourcesQuery) {
  const resources: Map<number, Resource> = new Map(
    query.resources.map((r) => {
      const availability = r.availability?.reduce((acc: Availability, item) => {
        const weekdayMap: Record<string, keyof Availability> = {
          'Monday': 'mo',
          'Tuesday': 'tu',
          'Wednesday': 'we',
          'Thursday': 'th',
          'Friday': 'fr',
          'Saturday': 'sa',
          'Sunday': 'su'
        };
        const weekday = weekdayMap[item.weekday];
        if (weekday) {
          acc[weekday] = item.duration;
        }
        return acc;
      }, {} as Availability) || null;
      
      return [
        r.dbId,
        {
          dbId: r.dbId,
          name: r.name,
          timezone: r.timezone,
          added: new Date(r.added),
          removed: r.removed == null ? null : new Date(r.removed),
          holidayId: r.holiday?.dbId ?? null,
          availability,
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
    holidayId: resource.value.holidayId ?? null,
    availability: Object.entries(resource.value.availability).map(([key, value]) => {
      const weekdayMap = {
        'mo': Weekday.Monday,
        'tu': Weekday.Tuesday,
        'we': Weekday.Wednesday,
        'th': Weekday.Thursday,
        'fr': Weekday.Friday,
        'sa': Weekday.Saturday,
        'su': Weekday.Sunday
      } as const;
      const weekday = weekdayMap[key as keyof typeof weekdayMap];
      if (!weekday) {
        throw new Error(`Invalid weekday key: ${key}`);
      }
      return {
        weekday,
        duration: value
      };
    }),
  };
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
