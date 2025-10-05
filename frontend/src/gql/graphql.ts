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
  /**
   * Date in the proleptic Gregorian calendar (without time zone).
   *
   * Represents a description of the date (as used for birthdays, for example).
   * It cannot represent an instant on the time-line.
   *
   * [`LocalDate` scalar][1] compliant.
   *
   * See also [`chrono::NaiveDate`][2] for details.
   *
   * [1]: https://graphql-scalars.dev/docs/scalars/local-date
   * [2]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html
   */
  LocalDate: { input: any; output: any; }
};

export type Allocation = {
  __typename?: 'Allocation';
  dbId: Scalars['Int']['output'];
  end: Scalars['DateTime']['output'];
  resources: Array<Resource>;
  start: Scalars['DateTime']['output'];
  task: Task;
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

export enum CalculationState {
  Calculating = 'CALCULATING',
  Finished = 'FINISHED',
  Modified = 'MODIFIED'
}

export type CalculationUpdate = {
  __typename?: 'CalculationUpdate';
  plan?: Maybe<Plan>;
  state: CalculationState;
};

export type Country = {
  __typename?: 'Country';
  isocode: Scalars['String']['output'];
  name: Scalars['String']['output'];
  regions: Array<Region>;
};

export type Holiday = {
  __typename?: 'Holiday';
  country?: Maybe<Country>;
  dbId: Scalars['Int']['output'];
  entries: Array<HolidayEntry>;
  externalId: Scalars['String']['output'];
  name: Scalars['String']['output'];
  region?: Maybe<Region>;
};


export type HolidayEntriesArgs = {
  from: Scalars['LocalDate']['input'];
  until: Scalars['LocalDate']['input'];
};

export type HolidayEntry = {
  __typename?: 'HolidayEntry';
  date: Scalars['LocalDate']['output'];
  dbId: Scalars['Int']['output'];
  holiday: Holiday;
  name?: Maybe<Scalars['String']['output']>;
};

export type Interval = {
  __typename?: 'Interval';
  end: Scalars['DateTime']['output'];
  start: Scalars['DateTime']['output'];
};

export type Issue = {
  __typename?: 'Issue';
  code: IssueCode;
  dbId: Scalars['Int']['output'];
  description: Scalars['String']['output'];
  task?: Maybe<Task>;
  type: IssueType;
};

export enum IssueCode {
  DependencyLoop = 'DEPENDENCY_LOOP',
  HierarchyLoop = 'HIERARCHY_LOOP',
  MilestoneMissing = 'MILESTONE_MISSING',
  NoEffort = 'NO_EFFORT',
  NoSlotFound = 'NO_SLOT_FOUND',
  PredIssue = 'PRED_ISSUE',
  RequirementMissing = 'REQUIREMENT_MISSING',
  ResourceMissing = 'RESOURCE_MISSING',
  Unknown = 'UNKNOWN'
}

export enum IssueType {
  General = 'GENERAL',
  PlanningGeneral = 'PLANNING_GENERAL',
  PlanningTask = 'PLANNING_TASK',
  Task = 'TASK'
}

export type Mutation = {
  __typename?: 'Mutation';
  new: Mutation;
  /** Trigger a manual recalculation now */
  recalculateNow: Scalars['Boolean']['output'];
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

export type Plan = {
  __typename?: 'Plan';
  allocations: Array<Allocation>;
};

export type Query = {
  __typename?: 'Query';
  countries: Array<Country>;
  country?: Maybe<Country>;
  currentPlan: Plan;
  getFromOpenHolidays?: Maybe<Holiday>;
  helloWorld: Scalars['String']['output'];
  issues: Array<Issue>;
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
  country: Country;
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
  combinedAvailability: Array<Interval>;
  dbId: Scalars['Int']['output'];
  holiday?: Maybe<Holiday>;
  name: Scalars['String']['output'];
  removed?: Maybe<Scalars['DateTime']['output']>;
  timezone: Scalars['String']['output'];
  vacation: Array<Vacation>;
};


export type ResourceCombinedAvailabilityArgs = {
  end: Scalars['DateTime']['input'];
  start: Scalars['DateTime']['input'];
};

export type ResourceConstraint = {
  __typename?: 'ResourceConstraint';
  entries: Array<ResourceConstraintEntry>;
  id: Scalars['Int']['output'];
  optional: Scalars['Boolean']['output'];
  speed: Scalars['Float']['output'];
};

export type ResourceConstraintEntry = {
  __typename?: 'ResourceConstraintEntry';
  id: Scalars['Int']['output'];
  resource: Resource;
};

export type ResourceConstraintEntryInput = {
  resourceId: Scalars['Int']['input'];
};

export type ResourceConstraintInput = {
  entries: Array<ResourceConstraintEntryInput>;
  optional: Scalars['Boolean']['input'];
  speed: Scalars['Float']['input'];
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
  calculationUpdate: CalculationUpdate;
};

export type Task = {
  __typename?: 'Task';
  allocations: Array<Allocation>;
  children: Array<Task>;
  dbId: Scalars['Int']['output'];
  description: Scalars['String']['output'];
  designation: TaskDesignation;
  earliestStart?: Maybe<Scalars['DateTime']['output']>;
  effort?: Maybe<Scalars['Float']['output']>;
  issues: Array<Issue>;
  parent?: Maybe<Task>;
  predecessors: Array<Task>;
  resourceConstraints: Array<ResourceConstraint>;
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
  resourceConstraints?: InputMaybe<Array<ResourceConstraintInput>>;
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

export type GetCountriesQueryVariables = Exact<{ [key: string]: never; }>;


export type GetCountriesQuery = { __typename?: 'Query', countries: Array<{ __typename?: 'Country', isocode: string, name: string }> };

export type GetRegionsQueryVariables = Exact<{
  isocode: Scalars['String']['input'];
}>;


export type GetRegionsQuery = { __typename?: 'Query', country?: { __typename?: 'Country', regions: Array<{ __typename?: 'Region', name: string, isocode: string }> } | null };

export type GetHolidayQueryVariables = Exact<{
  isocode: Scalars['String']['input'];
}>;


export type GetHolidayQuery = { __typename?: 'Query', getFromOpenHolidays?: { __typename?: 'Holiday', dbId: number, name: string, country?: { __typename?: 'Country', name: string, isocode: string } | null, region?: { __typename?: 'Region', name: string, isocode: string } | null } | null };

export type IssuesQueryVariables = Exact<{ [key: string]: never; }>;


export type IssuesQuery = { __typename?: 'Query', issues: Array<{ __typename?: 'Issue', dbId: number, code: IssueCode, description: string, type: IssueType, task?: { __typename?: 'Task', dbId: number } | null }> };

export type PlanQueryVariables = Exact<{ [key: string]: never; }>;


export type PlanQuery = { __typename?: 'Query', currentPlan: { __typename?: 'Plan', allocations: Array<{ __typename?: 'Allocation', dbId: number, start: any, end: any, task: { __typename?: 'Task', dbId: number }, resources: Array<{ __typename?: 'Resource', dbId: number }> }> } };

export type CalcUpdateSubscriptionVariables = Exact<{ [key: string]: never; }>;


export type CalcUpdateSubscription = { __typename?: 'Subscription', calculationUpdate: { __typename?: 'CalculationUpdate', state: CalculationState } };

export type RecalculateMutationVariables = Exact<{ [key: string]: never; }>;


export type RecalculateMutation = { __typename?: 'Mutation', recalculateNow: boolean };

export type ResourcesQueryVariables = Exact<{ [key: string]: never; }>;


export type ResourcesQuery = { __typename?: 'Query', resources: Array<{ __typename?: 'Resource', dbId: number, name: string, timezone: string, added: any, removed?: any | null, vacation: Array<{ __typename?: 'Vacation', dbId: number, from: any, until: any }>, holiday?: { __typename?: 'Holiday', dbId: number, name: string, country?: { __typename?: 'Country', name: string, isocode: string } | null, region?: { __typename?: 'Region', name: string, isocode: string } | null } | null, availability: Array<{ __typename?: 'Availability', weekday: Weekday, duration: number }> }> };

export type CombinedAvailabilityQueryVariables = Exact<{
  start: Scalars['DateTime']['input'];
  end: Scalars['DateTime']['input'];
}>;


export type CombinedAvailabilityQuery = { __typename?: 'Query', resources: Array<{ __typename?: 'Resource', dbId: number, combinedAvailability: Array<{ __typename?: 'Interval', start: any, end: any }> }> };

export type Resource_SaveMutationVariables = Exact<{
  resource: ResourceSaveInput;
}>;


export type Resource_SaveMutation = { __typename?: 'Mutation', resourceSave: { __typename?: 'Resource', dbId: number } };

export type Resource_DeleteMutationVariables = Exact<{
  resourceId: Scalars['Int']['input'];
}>;


export type Resource_DeleteMutation = { __typename?: 'Mutation', resourceDelete: boolean };

export type TasksQueryVariables = Exact<{ [key: string]: never; }>;


export type TasksQuery = { __typename?: 'Query', tasks: Array<{ __typename?: 'Task', dbId: number, title: string, description: string, designation: TaskDesignation, earliestStart?: any | null, scheduleTarget?: any | null, effort?: number | null, parent?: { __typename?: 'Task', dbId: number } | null, predecessors: Array<{ __typename?: 'Task', dbId: number }>, resourceConstraints: Array<{ __typename?: 'ResourceConstraint', optional: boolean, speed: number, entries: Array<{ __typename?: 'ResourceConstraintEntry', resource: { __typename?: 'Resource', dbId: number } }> }> }> };

export type Task_SaveMutationVariables = Exact<{
  task: TaskSaveInput;
}>;


export type Task_SaveMutation = { __typename?: 'Mutation', taskSave: { __typename?: 'Task', dbId: number } };

export type Task_DeleteMutationVariables = Exact<{
  taskId: Scalars['Int']['input'];
}>;


export type Task_DeleteMutation = { __typename?: 'Mutation', taskDelete: boolean };


export const GetCountriesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetCountries"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"countries"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"isocode"}},{"kind":"Field","name":{"kind":"Name","value":"name"}}]}}]}}]} as unknown as DocumentNode<GetCountriesQuery, GetCountriesQueryVariables>;
export const GetRegionsDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetRegions"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"isocode"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"country"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"isocode"},"value":{"kind":"Variable","name":{"kind":"Name","value":"isocode"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"regions"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"isocode"}}]}}]}}]}}]} as unknown as DocumentNode<GetRegionsQuery, GetRegionsQueryVariables>;
export const GetHolidayDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"GetHoliday"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"isocode"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"getFromOpenHolidays"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"isocode"},"value":{"kind":"Variable","name":{"kind":"Name","value":"isocode"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"country"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"isocode"}}]}},{"kind":"Field","name":{"kind":"Name","value":"region"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"isocode"}}]}}]}}]}}]} as unknown as DocumentNode<GetHolidayQuery, GetHolidayQueryVariables>;
export const IssuesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"issues"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"issues"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"code"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"type"}},{"kind":"Field","name":{"kind":"Name","value":"task"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]}}]} as unknown as DocumentNode<IssuesQuery, IssuesQueryVariables>;
export const PlanDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"plan"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"currentPlan"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"allocations"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"start"}},{"kind":"Field","name":{"kind":"Name","value":"end"}},{"kind":"Field","name":{"kind":"Name","value":"task"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}},{"kind":"Field","name":{"kind":"Name","value":"resources"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]}}]}}]} as unknown as DocumentNode<PlanQuery, PlanQueryVariables>;
export const CalcUpdateDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"subscription","name":{"kind":"Name","value":"calcUpdate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"calculationUpdate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"state"}}]}}]}}]} as unknown as DocumentNode<CalcUpdateSubscription, CalcUpdateSubscriptionVariables>;
export const RecalculateDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"recalculate"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"recalculateNow"}}]}}]} as unknown as DocumentNode<RecalculateMutation, RecalculateMutationVariables>;
export const ResourcesDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"resources"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resources"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"timezone"}},{"kind":"Field","name":{"kind":"Name","value":"added"}},{"kind":"Field","name":{"kind":"Name","value":"removed"}},{"kind":"Field","name":{"kind":"Name","value":"vacation"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"from"}},{"kind":"Field","name":{"kind":"Name","value":"until"}}]}},{"kind":"Field","name":{"kind":"Name","value":"holiday"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"country"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"isocode"}}]}},{"kind":"Field","name":{"kind":"Name","value":"region"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"name"}},{"kind":"Field","name":{"kind":"Name","value":"isocode"}}]}}]}},{"kind":"Field","name":{"kind":"Name","value":"availability"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"weekday"}},{"kind":"Field","name":{"kind":"Name","value":"duration"}}]}}]}}]}}]} as unknown as DocumentNode<ResourcesQuery, ResourcesQueryVariables>;
export const CombinedAvailabilityDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"combinedAvailability"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"start"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"DateTime"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"end"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"DateTime"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resources"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"combinedAvailability"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"start"},"value":{"kind":"Variable","name":{"kind":"Name","value":"start"}}},{"kind":"Argument","name":{"kind":"Name","value":"end"},"value":{"kind":"Variable","name":{"kind":"Name","value":"end"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"start"}},{"kind":"Field","name":{"kind":"Name","value":"end"}}]}}]}}]}}]} as unknown as DocumentNode<CombinedAvailabilityQuery, CombinedAvailabilityQueryVariables>;
export const Resource_SaveDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"resource_save"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"resource"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ResourceSaveInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resourceSave"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"resource"},"value":{"kind":"Variable","name":{"kind":"Name","value":"resource"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]} as unknown as DocumentNode<Resource_SaveMutation, Resource_SaveMutationVariables>;
export const Resource_DeleteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"resource_delete"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"resourceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resourceDelete"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"resourceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"resourceId"}}}]}]}}]} as unknown as DocumentNode<Resource_DeleteMutation, Resource_DeleteMutationVariables>;
export const TasksDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"tasks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tasks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"designation"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}},{"kind":"Field","name":{"kind":"Name","value":"predecessors"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}},{"kind":"Field","name":{"kind":"Name","value":"earliestStart"}},{"kind":"Field","name":{"kind":"Name","value":"scheduleTarget"}},{"kind":"Field","name":{"kind":"Name","value":"effort"}},{"kind":"Field","name":{"kind":"Name","value":"designation"}},{"kind":"Field","name":{"kind":"Name","value":"resourceConstraints"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"optional"}},{"kind":"Field","name":{"kind":"Name","value":"speed"}},{"kind":"Field","name":{"kind":"Name","value":"entries"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"resource"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]}}]}}]}}]} as unknown as DocumentNode<TasksQuery, TasksQueryVariables>;
export const Task_SaveDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"task_save"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"task"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"TaskSaveInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"taskSave"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"task"},"value":{"kind":"Variable","name":{"kind":"Name","value":"task"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]} as unknown as DocumentNode<Task_SaveMutation, Task_SaveMutationVariables>;
export const Task_DeleteDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"task_delete"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"taskId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"Int"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"taskDelete"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"taskId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"taskId"}}}]}]}}]} as unknown as DocumentNode<Task_DeleteMutation, Task_DeleteMutationVariables>;