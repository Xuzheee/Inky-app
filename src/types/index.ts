export type TaskCategory = 'work' | 'study' | 'life' | 'idea';

export type TaskPriority = 'high' | 'medium' | 'low';

export type Mood = '好' | '一般' | '烦';

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

export interface LevelConfig {
  level: 1 | 2 | 3;
  title: string;
  requiredXp: number;
}

export interface LevelProgress {
  current: LevelConfig;
  next: LevelConfig | null;
  progressPercent: number;
  xpIntoLevel: number;
  xpForNextLevel: number | null;
}
