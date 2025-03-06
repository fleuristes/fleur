import { Loader } from '@/components/ui/loader';
import { AppListItem } from '@/components/app/AppListItem';
import { useApps } from '@/appRegistry';

import type { App } from "@/types/components/app";
interface HomeProps {
  configuredApps: { [key: string]: boolean };
  installedApps: { [key: string]: boolean };
  onAppSelect: (app: App) => void;
  onInstallationChange: (appName: string, isInstalled: boolean) => void;
}

export function Home({
  configuredApps,
  installedApps,
  onAppSelect,
  onInstallationChange,
}: HomeProps) {
  const { apps, isLoading } = useApps();
  
  if (isLoading) {
    return (
      <div className="flex items-center justify-center min-h-[400px]">
        <Loader className="text-gray-800 dark:text-gray-200" />
      </div>
    );
  }

  return (
    <section className="flex flex-col gap-4">
      <div className="flex items-center justify-start">
        <h2 className="w-[100px] text-4xl font-bold text-gray-900 dark:text-gray-100">Apps</h2>
      </div>
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-1">
        {apps.map((app) => (
          <AppListItem
            key={app.name}
            app={app}
            isConfigured={configuredApps[app.name]}
            isInstalled={installedApps[app.name]}
            onAppSelect={onAppSelect}
            onInstallationChange={(isInstalled) =>
              onInstallationChange(app.name, isInstalled)
            }
          />
        ))}
      </div>
    </section>
  );
}
