export type TaskCategory = 'work' | 'study' | 'life' | 'idea';

export type TaskPriority = 'high' | 'medium' | 'low';

export type Mood = '好' | '一般' | '烦';

export type InboxItemStatus = 'pending' | 'converted' | 'archived' | 'deleted';

export type ViewState = 'main' | 'focus';

export type OverlayState = 'none' | 'category' | 'ai-parse' | 'break' | 'level-up' | 'settings';

export type PetMood = 'idle' | 'happy' | 'think' | 'rest';

export interface Task {
  id: string;
  title: string;
  category: TaskCategory;
  priority: TaskPriority;
  completed: boolean;
  due: string | null;
  completedPomodoros: number;
}

export interface InboxItem {
  id: string;
  text: string;
  createdAt: string;
  status: InboxItemStatus;
  convertedTaskId: string | null;
  date: string;
}

export interface LevelConfig {
  level: 1 | 2 | 3;
  requiredXp: number;
}

export interface LevelProgress {
  current: LevelConfig;
  next: LevelConfig | null;
  progressPercent: number;
  xpIntoLevel: number;
  xpForNextLevel: number | null;
}
