import { invoke } from '@tauri-apps/api/core';
import type { Mood, Task, TaskCategory, TaskPriority } from '../types';

const taskCategories = new Set<TaskCategory>(['work', 'study', 'life', 'idea']);
const taskPriorities = new Set<TaskPriority>(['high', 'medium', 'low']);
const moods = new Set<Mood>(['好', '一般', '烦']);

export interface FocusFlowPersistedState {
  version: 1;
  tasks: Task[];
  mood: Mood;
  xp: number;
  petName: string;
}

export const defaultFocusFlowState: FocusFlowPersistedState = {
  version: 1,
  petName: 'Inky',
  tasks: [
    { id: '1', title: '回复导师邮件', category: 'work', priority: 'high', completed: false, due: '今天 16:00', completedPomodoros: 0 },
    { id: '2', title: '整理读书笔记', category: 'study', priority: 'medium', completed: false, due: null, completedPomodoros: 0 },
    { id: '3', title: '买明天早饭食材', category: 'life', priority: 'low', completed: true, due: '明早', completedPomodoros: 0 },
  ],
  mood: '好',
  xp: 10,
};

export function cloneDefaultFocusFlowState(): FocusFlowPersistedState {
  return {
    ...defaultFocusFlowState,
    tasks: defaultFocusFlowState.tasks.map((task) => ({ ...task })),
  };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function normalizeTask(value: unknown): Task | null {
  if (!isRecord(value)) {
    return null;
  }

  if (
    typeof value.id !== 'string' ||
    typeof value.title !== 'string' ||
    typeof value.completed !== 'boolean' ||
    !(typeof value.due === 'string' || value.due === null) ||
    typeof value.category !== 'string' ||
    !taskCategories.has(value.category as TaskCategory) ||
    typeof value.priority !== 'string' ||
    !taskPriorities.has(value.priority as TaskPriority)
  ) {
    return null;
  }

  const completedPomodoros =
    typeof value.completedPomodoros === 'number' &&
    Number.isFinite(value.completedPomodoros) &&
    value.completedPomodoros >= 0
      ? Math.floor(value.completedPomodoros)
      : 0;

  return {
    id: value.id,
    title: value.title,
    category: value.category as TaskCategory,
    priority: value.priority as TaskPriority,
    completed: value.completed,
    due: value.due,
    completedPomodoros,
  };
}

function normalizeState(value: unknown): FocusFlowPersistedState | null {
  if (
    !isRecord(value) ||
    value.version !== 1 ||
    !Array.isArray(value.tasks) ||
    typeof value.mood !== 'string' ||
    !moods.has(value.mood as Mood) ||
    typeof value.xp !== 'number' ||
    !Number.isFinite(value.xp) ||
    value.xp < 0
  ) {
    return null;
  }

  const tasks = value.tasks.map(normalizeTask);

  if (tasks.some((task) => task === null)) {
    return null;
  }

  return {
    version: 1,
    tasks: tasks as Task[],
    mood: value.mood as Mood,
    xp: Math.floor(value.xp),
    petName:
      typeof value.petName === 'string' && value.petName.trim() && value.petName.trim() !== '小章章'
        ? value.petName.trim()
        : defaultFocusFlowState.petName,
  };
}

export async function loadFocusFlowState(): Promise<FocusFlowPersistedState> {
  try {
    return normalizeState(await invoke('load_focus_flow_state')) ?? cloneDefaultFocusFlowState();
  } catch {
    return cloneDefaultFocusFlowState();
  }
}

export async function saveFocusFlowState(state: Pick<FocusFlowPersistedState, 'tasks' | 'mood' | 'xp' | 'petName'>) {
  try {
    await invoke('save_focus_flow_state', {
      payload: {
        version: 1,
        tasks: state.tasks,
        mood: state.mood,
        xp: Math.max(0, Math.floor(state.xp)),
        petName: state.petName.trim() || defaultFocusFlowState.petName,
      } satisfies FocusFlowPersistedState,
    });
  } catch {
    // Persistence remains best-effort so UI actions are not blocked by disk errors.
  }
}
