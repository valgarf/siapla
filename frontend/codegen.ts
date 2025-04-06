import type { CodegenConfig } from '@graphql-codegen/cli';

const config: CodegenConfig = {
  overwrite: true,
  schema: 'http://localhost:8880/graphql',
  documents: 'src/**/*',
  generates: {
    'src/gql/': {
      preset: 'client',
      plugins: [],
      config: {
        scalars: {
          // Recommended ID scalar type for clients:
          ID: {
            input: 'string | number',
            output: 'string',
          },
          // Setting custom scalar type:
          LocalDateTime: {
            input: 'string', // this means our server can take CustomScalar as string
            output: 'string', // this means our server will return CustomScalar as number
          },
        },
      },
    },
    './graphql.schema.json': {
      plugins: ['introspection'],
    },
    'src/gql/schema.graphql': {
      plugins: ['schema-ast'],
    },
  },
};

export default config;
