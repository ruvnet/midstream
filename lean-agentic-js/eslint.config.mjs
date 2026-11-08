// ESLint flat config (ESLint 9+ dropped .eslintrc; ESLint 10 removed it).
import tsPlugin from '@typescript-eslint/eslint-plugin';

export default [
  { ignores: ['dist/**', 'node_modules/**'] },
  ...tsPlugin.configs['flat/recommended'],
  {
    rules: {
      '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
      // Public metadata/environment types are intentionally open-ended.
      '@typescript-eslint/no-explicit-any': 'warn',
    },
  },
];
