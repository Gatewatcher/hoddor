module.exports = {
  env: {
    node: true,
  },
  parser: '@typescript-eslint/parser',
  extends: [
    'eslint:recommended',
    'plugin:react/recommended',
    'plugin:react/jsx-runtime',
    'plugin:react-hooks/recommended',
    'plugin:@typescript-eslint/recommended',
  ],
  settings: {
    react: {
      version: 'detect',
    },
  },
  rules: {
    '@typescript-eslint/ban-types': 'off',
    '@typescript-eslint/explicit-module-boundary-types': 'off',
    '@typescript-eslint/no-empty-function': 'off',
    '@typescript-eslint/no-this-alias': 'off',
    '@typescript-eslint/no-namespace': 'off',
    'jsx-quotes': ['warn', 'prefer-double'],
    'no-case-declarations': 'off',
    'no-prototype-builtins': 'off',
    'no-useless-escape': 'off',
    quotes: [
      'warn',
      'single',
      { allowTemplateLiterals: true, avoidEscape: true },
    ],
    'react/display-name': 'off',
    'react/jsx-uses-react': 'off',
    'react/react-in-jsx-scope': 'off',
    semi: ['warn', 'always'],
  },
};
