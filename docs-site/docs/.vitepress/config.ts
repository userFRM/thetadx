import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'ThetaDataDx',
  description: 'Direct-wire SDK for ThetaData market data',
  base: '/ThetaDataDx/',
  cleanUrls: true,
  ignoreDeadLinks: true,

  head: [
    ['link', { rel: 'icon', type: 'image/svg+xml', href: '/logo.svg' }],
    ['meta', { name: 'theme-color', content: '#3b82f6' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'ThetaDataDx' }],
    ['meta', { property: 'og:description', content: 'Direct-wire SDK for ThetaData market data' }],
  ],

  themeConfig: {
    logo: '/logo.svg',
    siteTitle: 'ThetaDataDx',

    nav: [
      { text: 'Guide', link: '/getting-started' },
      { text: 'API Reference', link: '/api-reference' },
      { text: 'Tools', link: '/tools/cli' },
      { text: 'Changelog', link: '/changelog' },
      {
        text: 'GitHub',
        link: 'https://github.com/userFRM/thetadatadx',
      },
    ],

    sidebar: [
      {
        text: 'Guide',
        collapsed: false,
        items: [
          { text: 'Getting Started', link: '/getting-started' },
          { text: 'Historical Data', link: '/historical' },
          { text: 'Real-Time Streaming', link: '/streaming' },
          { text: 'Options & Greeks', link: '/options' },
          { text: 'Configuration', link: '/configuration' },
          { text: 'Jupyter Notebooks', link: '/notebooks' },
        ],
      },
      {
        text: 'Reference',
        collapsed: false,
        items: [
          { text: 'API Reference', link: '/api-reference' },
        ],
      },
      {
        text: 'Tools',
        collapsed: false,
        items: [
          { text: 'CLI', link: '/tools/cli' },
          { text: 'MCP Server', link: '/tools/mcp' },
          { text: 'REST Server', link: '/tools/server' },
        ],
      },
      {
        text: 'Project',
        collapsed: true,
        items: [
          { text: 'Changelog', link: '/changelog' },
        ],
      },
    ],

    socialLinks: [
      { icon: 'github', link: 'https://github.com/userFRM/thetadatadx' },
    ],

    search: {
      provider: 'local',
    },

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright 2024-present ThetaDataDx Contributors',
    },

    editLink: {
      pattern: 'https://github.com/userFRM/thetadatadx/edit/main/docs-site/docs/:path',
      text: 'Edit this page on GitHub',
    },
  },
})
