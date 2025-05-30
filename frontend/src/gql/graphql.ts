/* eslint-disable */
import { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
export type Maybe<T> = T | null;
export type InputMaybe<T> = Maybe<T>;
export type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
export type MakeOptional<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]?: Maybe<T[SubKey]> };
export type MakeMaybe<T, K extends keyof T> = Omit<T, K> & { [SubKey in K]: Maybe<T[SubKey]> };
export type MakeEmpty<T extends { [key: string]: unknown }, K extends keyof T> = { [_ in K]?: never };
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
/** All built-in and custom scalars, mapped to their actual values */
export type Scalars = {
  ID: { input: string; output: string; }
  String: { input: string; output: string; }
  Boolean: { input: boolean; output: boolean; }
  Int: { input: number; output: number; }
  Float: { input: number; output: number; }
  /**
   * Date in the proleptic Gregorian calendar (without time zone).
   *
   * Represents a description of the date (as used for birthdays, for example).
   * It cannot represent an instant on the time-line.
   *
   * [`Date` scalar][1] compliant.
   *
   * See also [`chrono::NaiveDate`][2] for details.
   *
   * [1]: https://graphql-scalars.dev/docs/scalars/date
   * [2]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html
   */
  Date: { input: any; output: any; }
  /**
   * Combined date and time (with time zone) in [RFC 3339][0] format.
   *
   * Represents a description of an exact instant on the time-line (such as the
   * instant that a user account was created).
   *
   * [`DateTime` scalar][1] compliant.
   *
   * See also [`chrono::DateTime`][2] for details.
   *
   * [0]: https://datatracker.ietf.org/doc/html/rfc3339#section-5
   * [1]: https://graphql-scalars.dev/docs/scalars/date-time
   * [2]: https://docs.rs/chrono/latest/chrono/struct.DateTime.html
   */
  DateTime: { input: any; output: any; }
};

export type Availability = {
  __typename?: 'Availability';
  dbId: Scalars['Int']['output'];
  duration: Scalars['Int']['output'];
  resource: Resource;
  weekday: Weekday;
};

export type AvailabilityInput = {
  duration: Scalars['Int']['input'];
  weekday: Weekday;
};

export type Country = {
  __typename?: 'Country';
  isocode: Scalars['String']['output'];
  name: Scalars['String']['output'];
  regions: Array<Region>;
};

export type Holiday = {
  __typename?: 'Holiday';
  dbId: Scalars['Int']['output'];
  entries: Array<HolidayEntry>;
  externalId: Scalars['String']['output'];
  name: Scalars['String']['output'];
};


export type HolidayEntriesArgs = {
  from: Scalars['Date']['input'];
  until: Scalars['Date']['input'];
};

export type HolidayEntry = {
  __typename?: 'HolidayEntry';
  date: Scalars['Date']['output'];
  dbId: Scalars['Int']['output'];
  holiday: Holiday;
  name?: Maybe<Scalars['String']['output']>;
};

export type Mutation = {
  __typename?: 'Mutation';
  new: Mutation;
  resourceDelete: Scalars['Boolean']['output'];
  resourceSave: Resource;
  taskDelete: Scalars['Boolean']['output'];
  taskSave: Task;
};


export type MutationResourceDeleteArgs = {
  resourceId: Scalars['Int']['input'];
};


export type MutationResourceSaveArgs = {
  resource: ResourceSaveInput;
};


export type MutationTaskDeleteArgs = {
  taskId: Scalars['Int']['input'];
};


export type MutationTaskSaveArgs = {
  task: TaskSaveInput;
};

export type Query = {
  __typename?: 'Query';
  countries: Array<Country>;
  country?: Maybe<Country>;
  getFromOpenHolidays?: Maybe<Holiday>;
  helloWorld: Scalars['String']['output'];
  region?: Maybe<Region>;
  resources: Array<Resource>;
  tasks: Array<Task>;
};


export type QueryCountryArgs = {
  isocode: Scalars['String']['input'];
};


export type QueryGetFromOpenHolidaysArgs = {
  isocode: Scalars['String']['input'];
};


export type QueryRegionArgs = {
  isocode: Scalars['String']['input'];
};

export type Region = {
  __typename?: 'Region';
  countryName: Scalars['String']['output'];
  holiday: Holiday;
  isocode: Scalars['String']['output'];
  name: Scalars['String']['output'];
  regionName: Scalars['String']['output'];
};

export type Resource = {
  __typename?: 'Resource';
  added: Scalars['DateTime']['output'];
  availability: Array<Availability>;
  dbId: Scalars['Int']['output'];
  holiday?: Maybe<Holiday>;
  name: Scalars['String']['output'];
  removed?: Maybe<Scalars['DateTime']['output']>;
  timezone: Scalars['String']['output'];
  vacation: Array<Vacation>;
};

export type ResourceSaveInput = {
  added: Scalars['DateTime']['input'];
  addedVacations?: InputMaybe<Array<VacationInput>>;
  availability?: InputMaybe<Array<AvailabilityInput>>;
  dbId?: InputMaybe<Scalars['Int']['input']>;
  holidayId?: InputMaybe<Scalars['Int']['input']>;
  name: Scalars['String']['input'];
  removed?: InputMaybe<Scalars['DateTime']['input']>;
  removedVacations?: InputMaybe<Array<Scalars['Int']['input']>>;
  timezone: Scalars['String']['input'];
};

export type Subscription = {
  __typename?: 'Subscription';
  apiVersion: Scalars['String']['output'];
};

export type Task = {
  __typename?: 'Task';
  children: Array<Task>;
  dbId: Scalars['Int']['output'];
  description: Scalars['String']['output'];
  designation: TaskDesignation;
  earliestStart?: Maybe<Scalars['DateTime']['output']>;
  effort?: Maybe<Scalars['Float']['output']>;
  parent?: Maybe<Task>;
  predecessors: Array<Task>;
  scheduleTarget?: Maybe<Scalars['DateTime']['output']>;
  successors: Array<Task>;
  title: Scalars['String']['output'];
};

export enum TaskDesignation {
  Group = 'GROUP',
  Milestone = 'MILESTONE',
  Requirement = 'REQUIREMENT',
  Task = 'TASK'
}

export type TaskSaveInput = {
  children?: InputMaybe<Array<Scalars['Int']['input']>>;
  dbId?: InputMaybe<Scalars['Int']['input']>;
  description: Scalars['String']['input'];
  designation: TaskDesignation;
  earliestStart?: InputMaybe<Scalars['DateTime']['input']>;
  effort?: InputMaybe<Scalars['Float']['input']>;
  parentId?: InputMaybe<Scalars['Int']['input']>;
  predecessors?: InputMaybe<Array<Scalars['Int']['input']>>;
  scheduleTarget?: InputMaybe<Scalars['DateTime']['input']>;
  successors?: InputMaybe<Array<Scalars['Int']['input']>>;
  title: Scalars['String']['input'];
};

export type Vacation = {
  __typename?: 'Vacation';
  dbId: Scalars['Int']['output'];
  from: Scalars['DateTime']['output'];
  until: Scalars['DateTime']['output'];
};

export type VacationInput = {
  from: Scalars['DateTime']['input'];
  until: Scalars['DateTime']['input'];
};

export enum Weekday {
  Friday = 'FRIDAY',
  Monday = 'MONDAY',
  Saturday = 'SATURDAY',
  Sunday = 'SUNDAY',
  Thursday = 'THURSDAY',
  Tuesday = 'TUESDAY',
  Wednesday = 'WEDNESDAY'
}

export type ResourcesQueryVariables = Exact<{ [key: string]: never; }>;


export type ResourcesQuery = { __typename?: 'Query', resources: Array<{ __typename?: 'Resource', dbId: number, name: string, timezone: string, added: any, removed?: any | null, holiday?: { __typename?: 'Holiday', dbId: number } | null, availability: Array<{ __typename?: 'Availability', weekday: Weekday, duration: number }> }> };

export type Resource_SaveMutationVariables = Exact<{
  resource: ResourceSaveInput;
}>;


export type Resource_SaveMutation = { __typename?: 'Mutation', resourceSave: { __typename?: 'Resource', dbId: number } };

export type Resource_DeleteMutationVariables = Exact<{
  resourceId: Scalars['Int']['input'];
}>;


export type Resource_DeleteMutation = { __typename?: 'Mutation', resourceDelete: boolean };

export type TasksQueryVariables = Exact<{ [key: string]: never; }>;


export type TasksQuery = { __typename?: 'Query', tasks: Array<{ __typename?: 'Task', dbId: number, title: string, description: string, designation: TaskDesignation, earliestStart?: any | null, scheduleTarget?: any | null, effort?: number | null, parent?: { __typename?: 'Task', dbId: number } | null, predecessors: Array<{ __typename?: 'Task', dbId: number }> }> };

export type Task_SaveMutationVariables = Exact<{
  task: TaskSaveInput;
}>;


export type Task_SaveMutation = { __typename?: 'Mutation', taskSave: { __typename?: 'Task', dbId: number } };

export type Task_DeleteMutationVariables = Exact<{
  taskId: Scalars['Int']['input'];
}>;


export type Task_DeleteMutation = { __typename?: 'Mutation', taskDelete: boolean };


export const ResourcesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"resources"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resources"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"timezone"}},{"kind":"Field","name":{"kind":"Name","value":"added"}},{"kind":"Field","name":{"kind":"Name","value":"removed"}},{"kind":"Field","name":{"kind":"Name","value":"holiday"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}},{"kind":"Field","name":{"kind":"Name","value":"availability"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"weekday"}},{"kind":"Field","name":{"kind":"Name","value":"duration"}}]}}]}}]}}]} as unknown as DocumentNode<ResourcesQuery, ResourcesQueryVariables>;
export const Resource_SaveDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"resource_save"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"resource"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ResourceSaveInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resourceSave"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"resource"},"value":{"kind":"Variable","name":{"kind":"Name","value":"resource"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]} as unknown as DocumentNode<Resource_SaveMutation, Resource_SaveMutationVariables>;
export const Resource_DeleteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"resource_delete"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"resourceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resourceDelete"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"resourceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"resourceId"}}}]}]}}]} as unknown as DocumentNode<Resource_DeleteMutation, Resource_DeleteMutationVariables>;
export const TasksDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"tasks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tasks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"designation"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}},{"kind":"Field","name":{"kind":"Name","value":"predecessors"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}},{"kind":"Field","name":{"kind":"Name","value":"earliestStart"}},{"kind":"Field","name":{"kind":"Name","value":"scheduleTarget"}},{"kind":"Field","name":{"kind":"Name","value":"effort"}},{"kind":"Field","name":{"kind":"Name","value":"designation"}}]}}]}}]} as unknown as DocumentNode<TasksQuery, TasksQueryVariables>;
export const Task_SaveDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"task_save"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"task"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"TaskSaveInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"taskSave"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"task"},"value":{"kind":"Variable","name":{"kind":"Name","value":"task"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]} as unknown as DocumentNode<Task_SaveMutation, Task_SaveMutationVariables>;
export const Task_DeleteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"task_delete"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"taskId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"taskDelete"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"taskId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"taskId"}}}]}]}}]} as unknown as DocumentNode<Task_DeleteMutation, Task_DeleteMutationVariables>;