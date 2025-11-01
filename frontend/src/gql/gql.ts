/* eslint-disable */
import * as types from './graphql';
import { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';

/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel or swc plugin for production.
 * Learn more about it here: https://the-guild.dev/graphql/codegen/plugins/presets/preset-client#reducing-bundle-size
 */
type Documents = {
    "\n  query GetCountries {\n    countries {\n      isocode\n      name\n    }\n  }\n": typeof types.GetCountriesDocument,
    "\n    query GetRegions($isocode: String!) {\n      country(isocode: $isocode) {\n        regions {\n          name\n          isocode\n        }\n      }\n    }\n  ": typeof types.GetRegionsDocument,
    "\n  query GetHoliday($isocode: String!) {\n    getFromOpenHolidays(isocode: $isocode) {\n      dbId\n      name\n      country {\n        name\n        isocode\n      }\n      region {\n        name\n        isocode\n      }\n    }\n  }\n": typeof types.GetHolidayDocument,
    "\n  mutation bookingSave($dbId: Int, $taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {\n    bookingSave(dbId: $dbId, taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }\n  }\n": typeof types.BookingSaveDocument,
    "\n  mutation bookingDelete($dbId: Int!) { bookingDelete(dbId: $dbId) }\n": typeof types.BookingDeleteDocument,
    "\n  query issues {\n    issues {\n      dbId\n      code\n      description\n      type\n      task {\n        dbId\n      }\n    }\n  }\n": typeof types.IssuesDocument,
    "\n  query plan {\n    currentPlan {\n      allocations {\n        dbId\n        start\n        end\n        allocationType\n        final\n        task { dbId }\n        resources { dbId }\n      }\n    }\n  }\n": typeof types.PlanDocument,
    "\n  subscription calcUpdate {\n    calculationUpdate {\n      state\n    }\n  }\n": typeof types.CalcUpdateDocument,
    "\n  mutation recalculate { recalculateNow }\n": typeof types.RecalculateDocument,
    "\n  query resources {\n    resources {\n      dbId\n      name\n      timezone\n      added\n      removed\n      vacation {\n        dbId\n        from\n        until\n      }\n      holiday {\n        dbId\n        name\n        country {\n          name\n          isocode\n        }\n        region {\n          name\n          isocode\n        }\n      }\n      availability {\n        weekday\n        duration\n      }\n    }\n  }\n": typeof types.ResourcesDocument,
    "\n  query combinedAvailability($start: DateTime!, $end: DateTime!) {\n    resources {\n      dbId\n      combinedAvailability(start: $start, end: $end) {\n        start\n        end\n      }\n    }\n  }\n": typeof types.CombinedAvailabilityDocument,
    "\n  mutation resource_save($resource: ResourceSaveInput!) {\n    resourceSave(resource: $resource) {\n      dbId\n    }\n  }\n": typeof types.Resource_SaveDocument,
    "\n  mutation resource_delete($resourceId: Int!) {\n    resourceDelete(resourceId: $resourceId)\n  }\n": typeof types.Resource_DeleteDocument,
    "\n  query tasks {\n    tasks {\n      dbId\n      title\n      description\n      designation\n      parent {\n        dbId\n      }\n      predecessors {\n        dbId\n      }\n      earliestStart\n      scheduleTarget\n      effort\n      designation\n      resourceConstraints {\n        optional\n        speed\n        entries {\n          resource {\n            dbId\n          }\n        }\n      }\n    }\n  }\n": typeof types.TasksDocument,
    "\n  mutation task_save($task: TaskSaveInput!) {\n    taskSave(task: $task) {\n      dbId\n    }\n  }\n": typeof types.Task_SaveDocument,
    "\n  mutation task_delete($taskId: Int!) {\n    taskDelete(taskId: $taskId)\n  }\n": typeof types.Task_DeleteDocument,
};
const documents: Documents = {
    "\n  query GetCountries {\n    countries {\n      isocode\n      name\n    }\n  }\n": types.GetCountriesDocument,
    "\n    query GetRegions($isocode: String!) {\n      country(isocode: $isocode) {\n        regions {\n          name\n          isocode\n        }\n      }\n    }\n  ": types.GetRegionsDocument,
    "\n  query GetHoliday($isocode: String!) {\n    getFromOpenHolidays(isocode: $isocode) {\n      dbId\n      name\n      country {\n        name\n        isocode\n      }\n      region {\n        name\n        isocode\n      }\n    }\n  }\n": types.GetHolidayDocument,
    "\n  mutation bookingSave($dbId: Int, $taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {\n    bookingSave(dbId: $dbId, taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }\n  }\n": types.BookingSaveDocument,
    "\n  mutation bookingDelete($dbId: Int!) { bookingDelete(dbId: $dbId) }\n": types.BookingDeleteDocument,
    "\n  query issues {\n    issues {\n      dbId\n      code\n      description\n      type\n      task {\n        dbId\n      }\n    }\n  }\n": types.IssuesDocument,
    "\n  query plan {\n    currentPlan {\n      allocations {\n        dbId\n        start\n        end\n        allocationType\n        final\n        task { dbId }\n        resources { dbId }\n      }\n    }\n  }\n": types.PlanDocument,
    "\n  subscription calcUpdate {\n    calculationUpdate {\n      state\n    }\n  }\n": types.CalcUpdateDocument,
    "\n  mutation recalculate { recalculateNow }\n": types.RecalculateDocument,
    "\n  query resources {\n    resources {\n      dbId\n      name\n      timezone\n      added\n      removed\n      vacation {\n        dbId\n        from\n        until\n      }\n      holiday {\n        dbId\n        name\n        country {\n          name\n          isocode\n        }\n        region {\n          name\n          isocode\n        }\n      }\n      availability {\n        weekday\n        duration\n      }\n    }\n  }\n": types.ResourcesDocument,
    "\n  query combinedAvailability($start: DateTime!, $end: DateTime!) {\n    resources {\n      dbId\n      combinedAvailability(start: $start, end: $end) {\n        start\n        end\n      }\n    }\n  }\n": types.CombinedAvailabilityDocument,
    "\n  mutation resource_save($resource: ResourceSaveInput!) {\n    resourceSave(resource: $resource) {\n      dbId\n    }\n  }\n": types.Resource_SaveDocument,
    "\n  mutation resource_delete($resourceId: Int!) {\n    resourceDelete(resourceId: $resourceId)\n  }\n": types.Resource_DeleteDocument,
    "\n  query tasks {\n    tasks {\n      dbId\n      title\n      description\n      designation\n      parent {\n        dbId\n      }\n      predecessors {\n        dbId\n      }\n      earliestStart\n      scheduleTarget\n      effort\n      designation\n      resourceConstraints {\n        optional\n        speed\n        entries {\n          resource {\n            dbId\n          }\n        }\n      }\n    }\n  }\n": types.TasksDocument,
    "\n  mutation task_save($task: TaskSaveInput!) {\n    taskSave(task: $task) {\n      dbId\n    }\n  }\n": types.Task_SaveDocument,
    "\n  mutation task_delete($taskId: Int!) {\n    taskDelete(taskId: $taskId)\n  }\n": types.Task_DeleteDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 *
 *
 * @example
 * ```ts
 * const query = graphql(`query GetUser($id: ID!) { user(id: $id) { name } }`);
 * ```
 *
 * The query argument is unknown!
 * Please regenerate the types.
 */
export function graphql(source: string): unknown;

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query GetCountries {\n    countries {\n      isocode\n      name\n    }\n  }\n"): (typeof documents)["\n  query GetCountries {\n    countries {\n      isocode\n      name\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n    query GetRegions($isocode: String!) {\n      country(isocode: $isocode) {\n        regions {\n          name\n          isocode\n        }\n      }\n    }\n  "): (typeof documents)["\n    query GetRegions($isocode: String!) {\n      country(isocode: $isocode) {\n        regions {\n          name\n          isocode\n        }\n      }\n    }\n  "];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query GetHoliday($isocode: String!) {\n    getFromOpenHolidays(isocode: $isocode) {\n      dbId\n      name\n      country {\n        name\n        isocode\n      }\n      region {\n        name\n        isocode\n      }\n    }\n  }\n"): (typeof documents)["\n  query GetHoliday($isocode: String!) {\n    getFromOpenHolidays(isocode: $isocode) {\n      dbId\n      name\n      country {\n        name\n        isocode\n      }\n      region {\n        name\n        isocode\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation bookingSave($dbId: Int, $taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {\n    bookingSave(dbId: $dbId, taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }\n  }\n"): (typeof documents)["\n  mutation bookingSave($dbId: Int, $taskId: Int!, $start: DateTime!, $end: DateTime!, $resources: [Int!]!, $final: Boolean!) {\n    bookingSave(dbId: $dbId, taskId: $taskId, start: $start, end: $end, resources: $resources, final: $final) { dbId }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation bookingDelete($dbId: Int!) { bookingDelete(dbId: $dbId) }\n"): (typeof documents)["\n  mutation bookingDelete($dbId: Int!) { bookingDelete(dbId: $dbId) }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query issues {\n    issues {\n      dbId\n      code\n      description\n      type\n      task {\n        dbId\n      }\n    }\n  }\n"): (typeof documents)["\n  query issues {\n    issues {\n      dbId\n      code\n      description\n      type\n      task {\n        dbId\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query plan {\n    currentPlan {\n      allocations {\n        dbId\n        start\n        end\n        allocationType\n        final\n        task { dbId }\n        resources { dbId }\n      }\n    }\n  }\n"): (typeof documents)["\n  query plan {\n    currentPlan {\n      allocations {\n        dbId\n        start\n        end\n        allocationType\n        final\n        task { dbId }\n        resources { dbId }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription calcUpdate {\n    calculationUpdate {\n      state\n    }\n  }\n"): (typeof documents)["\n  subscription calcUpdate {\n    calculationUpdate {\n      state\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation recalculate { recalculateNow }\n"): (typeof documents)["\n  mutation recalculate { recalculateNow }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query resources {\n    resources {\n      dbId\n      name\n      timezone\n      added\n      removed\n      vacation {\n        dbId\n        from\n        until\n      }\n      holiday {\n        dbId\n        name\n        country {\n          name\n          isocode\n        }\n        region {\n          name\n          isocode\n        }\n      }\n      availability {\n        weekday\n        duration\n      }\n    }\n  }\n"): (typeof documents)["\n  query resources {\n    resources {\n      dbId\n      name\n      timezone\n      added\n      removed\n      vacation {\n        dbId\n        from\n        until\n      }\n      holiday {\n        dbId\n        name\n        country {\n          name\n          isocode\n        }\n        region {\n          name\n          isocode\n        }\n      }\n      availability {\n        weekday\n        duration\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query combinedAvailability($start: DateTime!, $end: DateTime!) {\n    resources {\n      dbId\n      combinedAvailability(start: $start, end: $end) {\n        start\n        end\n      }\n    }\n  }\n"): (typeof documents)["\n  query combinedAvailability($start: DateTime!, $end: DateTime!) {\n    resources {\n      dbId\n      combinedAvailability(start: $start, end: $end) {\n        start\n        end\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation resource_save($resource: ResourceSaveInput!) {\n    resourceSave(resource: $resource) {\n      dbId\n    }\n  }\n"): (typeof documents)["\n  mutation resource_save($resource: ResourceSaveInput!) {\n    resourceSave(resource: $resource) {\n      dbId\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation resource_delete($resourceId: Int!) {\n    resourceDelete(resourceId: $resourceId)\n  }\n"): (typeof documents)["\n  mutation resource_delete($resourceId: Int!) {\n    resourceDelete(resourceId: $resourceId)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query tasks {\n    tasks {\n      dbId\n      title\n      description\n      designation\n      parent {\n        dbId\n      }\n      predecessors {\n        dbId\n      }\n      earliestStart\n      scheduleTarget\n      effort\n      designation\n      resourceConstraints {\n        optional\n        speed\n        entries {\n          resource {\n            dbId\n          }\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query tasks {\n    tasks {\n      dbId\n      title\n      description\n      designation\n      parent {\n        dbId\n      }\n      predecessors {\n        dbId\n      }\n      earliestStart\n      scheduleTarget\n      effort\n      designation\n      resourceConstraints {\n        optional\n        speed\n        entries {\n          resource {\n            dbId\n          }\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation task_save($task: TaskSaveInput!) {\n    taskSave(task: $task) {\n      dbId\n    }\n  }\n"): (typeof documents)["\n  mutation task_save($task: TaskSaveInput!) {\n    taskSave(task: $task) {\n      dbId\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation task_delete($taskId: Int!) {\n    taskDelete(taskId: $taskId)\n  }\n"): (typeof documents)["\n  mutation task_delete($taskId: Int!) {\n    taskDelete(taskId: $taskId)\n  }\n"];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;