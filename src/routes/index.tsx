import { useEffect } from "react";
import { useTheme } from "next-themes";
import { invoke } from "@tauri-apps/api/core";
import { useStore } from "@tanstack/react-store";
import { createFileRoute } from "@tanstack/react-router";
import { appStore, loadApps, loadAppStatuses, updateAppInstallation } from "@/store/app";
import { updateTauriTheme } from "@/lib/update-tauri-theme";
import { Loader } from "../components/ui/loader";
import { Home } from "../components/app/home";

export const Route = createFileRoute("/")({
  component: Index,
});

function Index() {
  const appStatuses = useStore(appStore, (state) => state.appStatuses);
  const isLoadingStatuses = useStore(
    appStore,
    (state) => state.isLoadingStatuses
  );
  const isLoadingApps = useStore(appStore, (state) => state.isLoadingApps);
  const hasInitializedInstalledApps = useStore(
    appStore,
    (state) => state.hasInitializedInstalledApps
  );
  const { theme } = useTheme();

  useEffect(() => {
    if (theme === "light" || theme === "dark") {
      updateTauriTheme(theme);
    }
  }, [theme]);

  // Log app statuses when they change, but debounce to avoid spam
  useEffect(() => {
    if (!appStatuses) return;

    const timer = setTimeout(() => {
      invoke("log_from_frontend", {
        level: "info",
        message: `App statuses: ${JSON.stringify(appStatuses)}`,
      }).catch((error) => {
        console.error("Failed to log app statuses:", error);
      });
    }, 500); // Debounce for 500ms

    return () => clearTimeout(timer);
  }, [appStatuses]);

  // Initialize environment only once
  useEffect(() => {
    let mounted = true;

    const initializeEnvironment = async () => {
      if (!mounted || hasInitializedInstalledApps) return;

      try {
        await invoke("ensure_environment");

        // Sequential loading to prevent race conditions
        if (mounted) {
          await loadAppStatuses();
          if (mounted) {
            await loadApps();
            if (mounted) {
              appStore.setState((state) => ({
                ...state,
                hasInitializedInstalledApps: true,
              }));
            }
          }
        }
      } catch (error) {
        console.error("Failed to initialize environment:", error);
      }
    };

    initializeEnvironment();

    return () => {
      mounted = false;
    };
  }, []); // Empty dependency array since we only want this to run once

  const handleInstallationChange = (appName: string, isInstalled: boolean) => {
    updateAppInstallation(appName, isInstalled);
  };

  if (isLoadingStatuses || isLoadingApps || !appStatuses) {
    return (
      <div className="min-h-screen bg-white dark:bg-gray-900 flex items-center justify-center">
        <Loader className="text-gray-800 dark:text-gray-200" size={48} />
      </div>
    );
  }

  return (
    <Home
      configuredApps={appStatuses.configured ?? {}}
      installedApps={appStatuses.installed ?? {}}
      onAppSelect={() => {}}
      onInstallationChange={handleInstallationChange}
    />
  );
}
