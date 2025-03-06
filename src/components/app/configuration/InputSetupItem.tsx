import { Input } from '@/components/ui/input';

// InputSetupItem component for setup items of type "input"
interface InputSetupItemProps {
  label: string;
  placeholder?: string;
  value: string;
  onChange: (value: string) => void;
}

export function InputSetupItem({ label, placeholder, value, onChange }: InputSetupItemProps) {
  return (
    <div className="flex flex-col w-full gap-2">
      <p className="text-base font-medium">{label}</p>
      <div className="flex gap-2">
        <Input
          type="text"
          placeholder={placeholder}
          value={value}
          onChange={(e) => onChange(e.target.value)}
        />
      </div>
    </div>
  );
}

export type { InputSetupItemProps }; 