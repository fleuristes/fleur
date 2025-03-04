import type { App } from "@/types/components/app";

export const apps: App[] = [
  {
    name: "Browser",
    description: "Web browser",
    stars: 1000,
    icon: {
      type: "url",
      url: {
        light: `/servers/browser.svg`,
        dark: `/servers/browser.svg`,
      },
    },
    category: "Utilities",
    price: "Get",
    developer: "Google LLC",
  },
  {
    name: "Hacker News",
    description: "Hacker News",
    stars: 1000,
    icon: {
      type: "url",
      url: {
        light: `/servers/yc.svg`,
        dark: `/servers/yc.svg`,
      },
    },
    category: "Social",
    price: "Get",
    developer: "Y Combinator",
  },
  {
    name: "June",
    description: "June",
    stars: 1000,
    icon: {
      type: "url",
      url: {
        light: `/servers/june.svg`,
        dark: `/servers/june.svg`,
      },
    },
    category: "Analytics",
    price: "Get",
    developer: "June",
    envVars: [
      {
        name: "JUNE_API_URL",
        label: "June API URL",
        description: "June API URL",
      },
      {
        name: "JUNE_API_KEY",
        label: "June API Key",
        description: "June API Key",
      },
    ],
  },
  {
    name: "Linear",
    description: "Linear",
    stars: 1000,
    icon: {
      type: "url",
      url: {
        light: `/servers/linear-dark.svg`,
        dark: `/servers/linear-light.svg`,
      },
    },
    category: "Productivity",
    price: "Get",
    developer: "Linear",
    envVars: [
      {
        name: "LINEAR_API_KEY",
        label: "Linear API Key",
        description: "Your Linear API key for authentication",
      },
    ],
  },
  {
    name: "Gmail",
    description: "Email and messaging platform",
    stars: 1000,
    icon: {
      type: "url",
      url: {
        light: `/servers/gmail.svg`,
        dark: `/servers/gmail.svg`,
      },
    },
    category: "Productivity",
    price: "Free",
    developer: "Google LLC",
  },
  {
    name: "Google Calendar",
    description: "Schedule and organize events",
    stars: 1000,
    icon: {
      type: "url",
      url: {
        light: `/servers/gcal.svg`,
        dark: `/servers/gcal.svg`,
      },
    },
    category: "Productivity",
    price: "Free",
    developer: "Google LLC",
  },
] as const;
