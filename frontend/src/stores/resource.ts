import { graphql } from 'src/gql';
import { defineStore, acceptHMRUpdate } from 'pinia';
import { useMutation, useQuery } from '@vue/apollo-composable';
import type { ResourceSaveInput, ResourcesQuery } from 'src/gql/graphql';
import { computed, type Ref } from 'vue';
import { ResourceDialogData, TaskDialogData, useDialogStore } from './dialog';

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
      return [
        r.dbId,
        {
          dbId: r.dbId,
          name: r.name,
          timezone: r.timezone,
          added: new Date(r.added),
          removed: r.removed == null ? null : new Date(r.removed),
          holidayId: r.holiday?.dbId ?? null,
          availability: null,
        },
      ];
    }),
  );
  return resources;
}

function resourceToObj(task: Ref<ResourceInput>): ResourceSaveInput {
  const result: ResourceSaveInput = {
    dbId: task.value.dbId ?? null,
    name: task.value.name,
    timezone: task.value.timezone,
    added: task.value.added.toISOString(),
    removed: task.value.removed?.toISOString(),
    holidayId: task.value.holidayId ?? null,
    availability: [],
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
        dialog.replaceDialog(new TaskDialogData(dbId));
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
  import.meta.hot.accept(acceptHMRUpdate(useTaskStore, import.meta.hot));
}
