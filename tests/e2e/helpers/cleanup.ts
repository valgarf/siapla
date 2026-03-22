import { graphqlRequest } from "./graphql";

/**
 * Clean the database by calling the resetDatabase mutation.
 *
 * This hard-deletes all data from all tables and creates a fresh revision.
 * Much more reliable than trying to soft-delete individual entities,
 * which just creates new revisions without actually removing data.
 */
export async function cleanDatabase(): Promise<void> {
    await graphqlRequest<{ resetDatabase: boolean }>(
        `mutation ResetDatabase {
            resetDatabase
        }`,
    );
}
