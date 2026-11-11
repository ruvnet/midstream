// ESLint 8 config for midstream-cli (`npm run lint`, run by js-ci.yml).
module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  parserOptions: { ecmaVersion: 2020, sourceType: 'module' },
  plugins: ['@typescript-eslint'],
  extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended'],
  env: { node: true, es2020: true, jest: true },
  rules: {
    // The public API (AnalysisResult, MCP tool payloads, WASM handles) is
    // still typed with `any`; typing it is tracked as follow-up work, so
    // surface these as warnings rather than failing CI on existing code.
    '@typescript-eslint/no-explicit-any': 'warn',
    // Allow intentionally unused parameters when prefixed with `_`.
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
  },
};
