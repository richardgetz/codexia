import React from 'react';
import { Button } from './ui/button';
import { Textarea } from './ui/textarea';
import { Badge } from './ui/badge';
import { Send, AtSign, X } from 'lucide-react';
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from './ui/tooltip';
import { useChatInputStore } from '../stores/chatInputStore';

interface ChatInputProps {
  inputValue: string;
  onInputChange: (value: string) => void;
  onSendMessage: (message: string) => void;
  disabled?: boolean;
  isLoading?: boolean;
}

export const ChatInput: React.FC<ChatInputProps> = ({
  inputValue,
  onInputChange,
  onSendMessage,
  disabled = false,
  isLoading = false,
}) => {
  const {
    fileReferences,
    removeFileReference,
    clearFileReferences,
  } = useChatInputStore();

  const generateSmartPrompt = (): string => {
    if (fileReferences.length === 0) return '';
    
    // Use the accurate isDirectory flag
    const directories = fileReferences.filter(ref => ref.isDirectory);
    const files = fileReferences.filter(ref => !ref.isDirectory);
    
    const filePaths = fileReferences.map(ref => ref.relativePath).join(' ');
    
    if (directories.length > 0 && files.length === 0) {
      return directories.length === 1 
        ? `Read this folder: ${filePaths}`
        : `Read these folders: ${filePaths}`;
    } else if (files.length > 0 && directories.length === 0) {
      return files.length === 1 
        ? `Read this file: ${filePaths}`
        : `Read these files: ${filePaths}`;
    } else {
      // Mixed files and folders
      return `Read these files and folders: ${filePaths}`;
    }
  };

  const handleSendMessage = () => {
    if (!inputValue.trim() || isLoading) return;

    // Build message content with file references
    let messageContent = inputValue;
    if (fileReferences.length > 0) {
      const smartPrompt = generateSmartPrompt();
      messageContent = `${smartPrompt}\n\n${inputValue}`;
    }

    onSendMessage(messageContent);
    onInputChange('');
    clearFileReferences();
  };

  type Shortcut = { name: string; hint: string; template: string };
  const ALL_SHORTCUTS: Shortcut[] = [
    // Built-in slash commands (mirror Codex CLI/TUI)
    { name: 'model', hint: 'Choose a model preset (model + reasoning effort)', template: '/model' },
    { name: 'new', hint: 'Start a new chat during a conversation', template: '/new' },
    { name: 'init', hint: 'Create an AGENTS.md with instructions for Codex', template: '/init' },
    { name: 'compact', hint: 'Summarize conversation to reduce context size', template: '/compact' },
    { name: 'diff', hint: 'Show git diff (including untracked files)', template: '/diff' },
    { name: 'mention', hint: 'Mention a file', template: '/mention ' },
    { name: 'status', hint: 'Show current session configuration and token usage', template: '/status' },
    { name: 'mcp', hint: 'List configured MCP tools', template: '/mcp' },
    { name: 'logout', hint: 'Log out of Codex', template: '/logout' },
    { name: 'quit', hint: 'Exit Codex', template: '/quit' },

    // Helpful non-slash templates
    { name: 'fix', hint: 'Fix the failing code or tests', template: 'Fix the issue in this repo. Explain changes and apply a patch.' },
    { name: 'plan', hint: 'Create a step-by-step plan', template: 'Create a concise, step-by-step plan for the task at hand.' },
    { name: 'review', hint: 'Code review the changes', template: 'Review recent changes. Call out risks and improvements.' },
    { name: 'refactor', hint: 'Refactor for clarity or performance', template: 'Refactor the code to improve readability and performance while preserving behavior.' },
    { name: 'test', hint: 'Write or update tests', template: 'Write or update targeted tests for the changed code.' },
    { name: 'summarize', hint: 'Summarize the current state', template: 'Summarize the current project state and recent changes.' },
    { name: 'explain', hint: 'Explain selected code', template: 'Explain what this code does and any pitfalls.' },
    { name: 'docs', hint: 'Improve README or docs', template: 'Improve documentation. Propose concise updates to README or docs.' },
  ];

  const [showShortcuts, setShowShortcuts] = React.useState(false);
  const [filteredShortcuts, setFilteredShortcuts] = React.useState<Shortcut[]>(ALL_SHORTCUTS);
  const [activeShortcut, setActiveShortcut] = React.useState(0);

  const updateShortcuts = React.useCallback((value: string) => {
    if (value.startsWith('/')) {
      const q = value.slice(1).toLowerCase();
      const list = q
        ? ALL_SHORTCUTS.filter((s) => s.name.toLowerCase().includes(q) || s.hint.toLowerCase().includes(q))
        : ALL_SHORTCUTS;
      setFilteredShortcuts(list);
      setActiveShortcut(0);
      setShowShortcuts(true);
    } else {
      setShowShortcuts(false);
    }
  }, []);

  const onTextareaChange = (v: string) => {
    onInputChange(v);
    updateShortcuts(v);
  };

  const applyShortcut = (s: Shortcut) => {
    onInputChange(s.template);
    setShowShortcuts(false);
  };

  const handleKeyPress = (e: React.KeyboardEvent) => {
    if (showShortcuts) {
      if (e.key === 'ArrowDown') {
        e.preventDefault();
        setActiveShortcut((i) => Math.min(i + 1, Math.max(0, filteredShortcuts.length - 1)));
        return;
      }
      if (e.key === 'ArrowUp') {
        e.preventDefault();
        setActiveShortcut((i) => Math.max(i - 1, 0));
        return;
      }
      if (e.key === 'Escape') {
        e.preventDefault();
        setShowShortcuts(false);
        return;
      }
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault();
        const s = filteredShortcuts[activeShortcut];
        if (s) applyShortcut(s);
        return;
      }
    }

    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSendMessage();
    }
  };

  return (
    <div className="flex-shrink-0 border-t p-4 bg-white relative">
      {/* File references display */}
      {fileReferences.length > 0 && (
        <div className="mb-3 flex flex-wrap gap-2 items-center">
          <AtSign className="w-4 h-4 text-gray-500" />
          {fileReferences.map((ref) => (
            <TooltipProvider key={ref.path}>
              <Tooltip>
                <TooltipTrigger>
                  <Badge
                    variant="secondary"
                    className="flex items-center gap-1 cursor-pointer hover:bg-gray-200"
                  >
                    <span>{ref.name}</span>
                    <X
                      className="w-3 h-3 hover:bg-gray-300 rounded"
                      onClick={(e) => {
                        e.stopPropagation();
                        removeFileReference(ref.path);
                      }}
                    />
                  </Badge>
                </TooltipTrigger>
                <TooltipContent>
                  <p>{ref.relativePath}</p>
                </TooltipContent>
              </Tooltip>
            </TooltipProvider>
          ))}
          <Button
            variant="ghost"
            size="sm"
            onClick={clearFileReferences}
            className="h-6 px-2 text-xs text-gray-500 hover:text-gray-700"
          >
            Clear all
          </Button>
        </div>
      )}
      
      <div className="flex gap-2">
        <Textarea
          value={inputValue}
          onChange={(e) => onTextareaChange(e.target.value)}
          onKeyDown={handleKeyPress}
          placeholder="Type your message..."
          className="flex-1 min-h-[40px] max-h-[120px]"
          disabled={disabled || isLoading}
        />
        {showShortcuts && filteredShortcuts.length > 0 && (
          <div className="absolute bottom-16 left-4 w-[360px] border rounded-md bg-white shadow-lg z-20">
            <div className="max-h-64 overflow-auto py-1">
              {filteredShortcuts.map((s, idx) => (
                <div
                  key={s.name}
                  className={`px-3 py-2 cursor-pointer ${idx === activeShortcut ? 'bg-gray-100' : ''}`}
                  onMouseEnter={() => setActiveShortcut(idx)}
                  onMouseDown={(e) => {
                    e.preventDefault();
                    applyShortcut(s);
                  }}
                >
                  <div className="text-sm font-medium">/{s.name}</div>
                  <div className="text-xs text-gray-500">{s.hint}</div>
                </div>
              ))}
            </div>
          </div>
        )}
        <Button
          onClick={handleSendMessage}
          disabled={!inputValue.trim() || isLoading || disabled}
          size="sm"
          className="self-end"
        >
          <Send className="w-4 h-4" />
        </Button>
      </div>
    </div>
  );
};
