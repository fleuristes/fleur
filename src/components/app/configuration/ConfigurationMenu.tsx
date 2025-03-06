import { DialogFooter } from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { TextSetupItem } from './TextSetupItem';
import { InputSetupItem } from './InputSetupItem';

import type { Setup } from "@/types/components/app";
// ConfigurationMenu component for the entire configuration menu
interface ConfigurationMenuProps {
  appName: string;
  setup: Setup[];
  setupValues: Record<string, string>;
  onInputChange: (key: string, value: string) => void;
  onSave: () => Promise<void>;
  isLoading: boolean;
}

export function ConfigurationMenu({
  appName,
  setup,
  setupValues,
  onInputChange,
  onSave,
  isLoading,
}: ConfigurationMenuProps) {
  return (
    <div className="flex flex-col gap-6 h-full">
      <div className="flex-1">
        <h2 className="text-lg font-semibold mb-2">
          Configure {appName}
        </h2>
        <p className="mb-4 text-sm leading-relaxed text-zinc-600 dark:text-zinc-400">
          This app requires some setup to use. Please follow the steps
          below.
        </p>
        <div className="flex flex-col gap-4">
          {setup.map((setupItem) =>
            setupItem?.type === "text" ? (
              <TextSetupItem
                key={setupItem.label}
                label={setupItem.label}
                value={setupItem.value || ""}
              />
            ) : (
              <InputSetupItem
                key={setupItem.label}
                label={setupItem.label}
                placeholder={setupItem.placeholder}
                value={setupValues[setupItem.key || ""] || ""}
                onChange={(value) => onInputChange(setupItem.key || "", value)}
              />
            )
          )}
        </div>
      </div>
      <DialogFooter>
        <Button
          variant="secondary"
          onClick={onSave}
          disabled={isLoading}>
          {isLoading ? "Saving..." : "Save"}
        </Button>
      </DialogFooter>
    </div>
  );
}

export type { ConfigurationMenuProps }; 