import js from '@eslint/js'
import ts from 'typescript-eslint'
import svelte from 'eslint-plugin-svelte'
import globals from 'globals'

export default ts.config(
  // src/lib/ipc/ is generated from the Rust IPC contract (cargo test, S1) —
  // the Rust types are the source of truth, so it is not hand-linted.
  { ignores: ['dist/', 'src-tauri/', 'node_modules/', 'src/lib/ipc/'] },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs.recommended,
  ...svelte.configs.prettier,
  {
    languageOptions: {
      globals: { ...globals.browser },
    },
  },
  {
    files: ['**/*.svelte'],
    languageOptions: {
      parserOptions: { parser: ts.parser },
    },
  },
)
