import { openUrl } from '@tauri-apps/plugin-opener';
import { invoke } from '@tauri-apps/api/core';

interface TextSetupItemProps {
  label: string;
  value: string;
}

function isAppProtocolUrl(url: string): boolean {
  return /^(?!http|https|ftp)\w+:\/\//.test(url);
}

async function openUrlWithFallback(url: string) {
  if (isAppProtocolUrl(url)) {
    try {
      await invoke('open_system_url', { url });
    } catch (error) {
      console.error(`Failed to open app URL with system command: ${error}`);
      
      try {
        await openUrl(url);
      } catch (secondError) {
        console.error(`Failed to open app URL with openUrl: ${secondError}`);
        
        window.open(url, '_blank');
      }
    }
  } else {
    try {
      await openUrl(url);
    } catch (error) {
      console.error(`Failed to open URL: ${error}`);
      window.open(url, '_blank');
    }
  }
}

function parseMarkdownLinks(text: string) {
  if (!text) return [];

  const linkRegex = /\[([^\]]+)\]\(([^)]+)\)/g;
  
  const parts: (string | JSX.Element)[] = [];
  let lastIndex = 0;
  let match;
  
  while ((match = linkRegex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(text.substring(lastIndex, match.index));
    }
    
    const [fullMatch, linkText, linkUrl] = match;
    parts.push(
      <button 
        key={match.index} 
        onClick={() => openUrlWithFallback(linkUrl)}
        className="text-blue-600 hover:underline dark:text-blue-400 font-normal p-0 bg-transparent border-none cursor-pointer"
      >
        {linkText}
      </button>
    );
    
    lastIndex = match.index + fullMatch.length;
  }
  
  if (lastIndex < text.length) {
    parts.push(text.substring(lastIndex));
  }
  
  return parts;
}

export function TextSetupItem({ label, value }: TextSetupItemProps) {
  const parsedContent = parseMarkdownLinks(value);
  
  return (
    <div className="flex flex-col gap-2">
      <p className="text-base font-medium">{label}</p>
      <p className="text-sm leading-relaxed text-zinc-600 dark:text-zinc-400">
        {parsedContent.length > 0 ? parsedContent : value}
      </p>
    </div>
  );
}

export type { TextSetupItemProps }; 