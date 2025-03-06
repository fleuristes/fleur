import { LucideIcon } from 'lucide-react';

export interface App {
  name: string;
  description: string;
  icon: {
    type: "url" | "lucide";
    url?: {
      light: string;
      dark: string;
    };
    icon?: LucideIcon;
  };
  category: string;
  price: string;
  developer: string;
  sourceUrl?: string;
  features?: Feature[];
  setup?: Setup[];
  envVars?: EnvVar[];
}

interface Feature {
  name: string;
  description: string;
  prompt: string;
}

export interface Setup {
  label: string;
  type: "text" | "input";
  placeholder?: string;
  value?: string;
  key: string;
}

interface EnvVar {
  name: string;
  label: string;
  description: string;
}

export interface AppListItemProps {
  app: App;
  isConfigured: boolean;
  isInstalled: boolean;
  onInstallationChange: (isInstalled: boolean) => void;
}

export interface AppIconProps {
  app: App;
}

export interface AppInstallButtonProps {
  app: App;
  isConfigured: boolean;
  isInstalled: boolean;
  onInstallationChange: (isInstalled: boolean) => void;
  showConfigure?: boolean;
}
