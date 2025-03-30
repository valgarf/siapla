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
   * Combined date and time (without time zone) in `yyyy-MM-dd HH:mm:ss` format.
   *
   * See also [`chrono::NaiveDateTime`][1] for details.
   *
   * [1]: https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDateTime.html
   */
  LocalDateTime: { input: any; output: any; }
};

export type Mutation = {
  __typename?: 'Mutation';
  new: Mutation;
  taskSave: Task;
};


export type MutationTaskSaveArgs = {
  task: TaskSaveInput;
};

export type Query = {
  __typename?: 'Query';
  helloWorld: Scalars['String']['output'];
  tasks: Array<Task>;
};

export type Subscription = {
  __typename?: 'Subscription';
  apiVersion: Scalars['String']['output'];
};

export type Task = {
  __typename?: 'Task';
  dbId: Scalars['Int']['output'];
  description: Scalars['String']['output'];
  earliestStart?: Maybe<Scalars['LocalDateTime']['output']>;
  effort?: Maybe<Scalars['Float']['output']>;
  parent?: Maybe<Task>;
  scheduleTarget?: Maybe<Scalars['LocalDateTime']['output']>;
  title: Scalars['String']['output'];
};

export type TaskSaveInput = {
  dbId?: InputMaybe<Scalars['Int']['input']>;
  description: Scalars['String']['input'];
  earliesStart?: InputMaybe<Scalars['LocalDateTime']['input']>;
  effort?: InputMaybe<Scalars['Float']['input']>;
  parentId?: InputMaybe<Scalars['Int']['input']>;
  scheduleTarget?: InputMaybe<Scalars['LocalDateTime']['input']>;
  title: Scalars['String']['input'];
};

export type TasksQueryVariables = Exact<{ [key: string]: never; }>;


export type TasksQuery = { __typename?: 'Query', tasks: Array<{ __typename?: 'Task', dbId: number, title: string, description: string, parent?: { __typename?: 'Task', dbId: number } | null }> };

export type Task_SaveMutationVariables = Exact<{
  task: TaskSaveInput;
}>;


export type Task_SaveMutation = { __typename?: 'Mutation', taskSave: { __typename?: 'Task', dbId: number } };


export const TasksDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"query","name":{"kind":"Name","value":"tasks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"tasks"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}},{"kind":"Field","name":{"kind":"Name","value":"title"}},{"kind":"Field","name":{"kind":"Name","value":"description"}},{"kind":"Field","name":{"kind":"Name","value":"parent"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]}}]} as unknown as DocumentNode<TasksQuery, TasksQueryVariables>;
export const Task_SaveDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"task_save"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"task"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"TaskSaveInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"taskSave"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"task"},"value":{"kind":"Variable","name":{"kind":"Name","value":"task"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"dbId"}}]}}]}}]} as unknown as DocumentNode<Task_SaveMutation, Task_SaveMutationVariables>;