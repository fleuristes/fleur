import { useEffect, useState } from "react";
import { useTheme } from "next-themes";
import { AppIconProps } from "@/types/components/app";

export function AppIcon({ app }: AppIconProps) {
  const { theme } = useTheme();
  const [systemTheme, setSystemTheme] = useState<"dark" | "light">("light");

  useEffect(() => {
    // Function to check system theme
    const checkSystemTheme = () => {
      const isDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
      setSystemTheme(isDark ? "dark" : "light");
    };

    // Initial check
    checkSystemTheme();

    // Listen for system theme changes
    const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
    const listener = (e: MediaQueryListEvent) => {
      setSystemTheme(e.matches ? "dark" : "light");
    };

    mediaQuery.addEventListener("change", listener);

    return () => mediaQuery.removeEventListener("change", listener);
  }, [theme, systemTheme]);

  if (app.icon.type === "lucide" && app.icon.icon) {
    const Icon = app.icon.icon;
    return <Icon className="h-6 w-6" />;
  }

  if (app.icon.type === "url" && app.icon.url) {
    // Use system theme as fallback if theme is not explicitly set
    const effectiveTheme = theme === "system" ? systemTheme : theme;

    return (
      <div className="w-full h-full flex items-center justify-center p-2 bg-gray-50 dark:bg-zinc-800">
        <img
          src={
            effectiveTheme === "dark" ? app.icon.url.dark : app.icon.url.light
          }
          alt={app.name}
          className="w-full h-full object-contain"
        />
      </div>
    );
  }

  return null;
}
