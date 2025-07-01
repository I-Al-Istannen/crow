import { defineConfigWithVueTs, vueTsConfigs } from '@vue/eslint-config-typescript'
import oxlint from 'eslint-plugin-oxlint'
import pluginVue from 'eslint-plugin-vue'
import skipFormatting from '@vue/eslint-config-prettier/skip-formatting'

export default defineConfigWithVueTs([
  {
    name: 'app/files-to-lint',
    files: ['**/*.{ts,mts,tsx,vue}'],
  },

  {
    name: 'app/files-to-ignore',
    ignores: ['**/dist/**', '**/dist-ssr/**', '**/coverage/**'],
  },

  vueTsConfigs.strictTypeChecked,
  vueTsConfigs.stylisticTypeChecked,
  pluginVue.configs['flat/essential'],
  oxlint.configs['flat/recommended'],
  skipFormatting,
  {
    rules: {
      "@typescript-eslint/no-non-null-assertion": 'warn',
      '@typescript-eslint/no-unused-vars': [
        'warn',
        {
          argsIgnorePattern: '^_',
          destructuredArrayIgnorePattern: '^_',
          varsIgnorePattern: '^_',
        },
      ],
      'sort-imports': ['error'],
      'vue/multi-word-component-names': [
        'error',
        {
          ignores: [
            'Accordion',
            'Alert',
            'Badge',
            'Button',
            'Card',
            'Checkbox',
            'Command',
            'Dialog',
            'Input',
            'Label',
            'Popover',
            'Select',
            'Separator',
            'Sonner',
            'Table',
            'Tabs',
            'Textarea',
            'Tooltip',
          ],
        },
      ],
    },
  },
])
