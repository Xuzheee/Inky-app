import { invoke } from '@tauri-apps/api/core';
import { DEFAULT_PET_SET_ID, isPetSetId, type PetSetId } from '../components/PetRenderer/petConfig';
import type { InboxItem, InboxItemStatus, Mood, Task, TaskCategory, TaskPriority } from '../types';

const taskCategories = new Set<TaskCategory>(['work', 'study', 'life', 'idea']);
const taskPriorities = new Set<TaskPriority>(['high', 'medium', 'low']);
const moods = new Set<Mood>(['好', '一般', '烦']);
const inboxStatuses = new Set<InboxItemStatus>(['pending', 'converted', 'archived', 'deleted']);

export interface FocusFlowPersistedState {
  version: 4;
  tasks: Task[];
  inboxItems: InboxItem[];
  mood: Mood;
  xp: number;
  petName: string;
  petSetId: PetSetId;
  showFocusReturn: boolean;
  hasCompletedPetNaming: boolean;
}

export const defaultFocusFlowState: FocusFlowPersistedState = {
  version: 4,
  petName: 'Inky',
  tasks: [
    { id: '1', title: '回复导师邮件', category: 'work', priority: 'high', completed: false, due: '今天 16:00', completedPomodoros: 0 },
    { id: '2', title: '整理读书笔记', category: 'study', priority: 'medium', completed: false, due: null, completedPomodoros: 0 },
    { id: '3', title: '买明天早饭食材', category: 'life', priority: 'low', completed: true, due: '明早', completedPomodoros: 0 },
  ],
  inboxItems: [],
  mood: '好',
  xp: 10,
  petSetId: DEFAULT_PET_SET_ID,
  showFocusReturn: true,
  hasCompletedPetNaming: false,
};

export function cloneDefaultFocusFlowState(): FocusFlowPersistedState {
  return {
    ...defaultFocusFlowState,
    tasks: defaultFocusFlowState.tasks.map((task) => ({ ...task })),
    inboxItems: defaultFocusFlowState.inboxItems.map((item) => ({ ...item })),
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

function normalizeInboxItem(value: unknown): InboxItem | null {
  if (!isRecord(value)) {
    return null;
  }

  if (
    typeof value.id !== 'string' ||
    value.id.trim() === '' ||
    typeof value.text !== 'string' ||
    value.text.trim() === '' ||
    typeof value.createdAt !== 'string' ||
    value.createdAt.trim() === '' ||
    typeof value.status !== 'string' ||
    !inboxStatuses.has(value.status as InboxItemStatus) ||
    !(typeof value.convertedTaskId === 'string' || value.convertedTaskId === null) ||
    typeof value.date !== 'string' ||
    value.date.trim() === ''
  ) {
    return null;
  }

  if ((value.status === 'converted') !== (value.convertedTaskId !== null)) {
    return null;
  }

  return {
    id: value.id,
    text: value.text,
    createdAt: value.createdAt,
    status: value.status as InboxItemStatus,
    convertedTaskId: value.convertedTaskId,
    date: value.date,
  };
}

function normalizeState(value: unknown): FocusFlowPersistedState | null {
  if (
    !isRecord(value) ||
    !(value.version === 3 || value.version === 4) ||
    !Array.isArray(value.tasks) ||
    !Array.isArray(value.inboxItems) ||
    typeof value.mood !== 'string' ||
    !moods.has(value.mood as Mood) ||
    typeof value.xp !== 'number' ||
    !Number.isFinite(value.xp) ||
    value.xp < 0
  ) {
    return null;
  }

  const tasks = value.tasks.map(normalizeTask);
  const inboxItems = value.inboxItems.map(normalizeInboxItem);

  if (tasks.some((task) => task === null) || inboxItems.some((item) => item === null)) {
    return null;
  }

  return {
    version: 4,
    tasks: tasks as Task[],
    inboxItems: inboxItems as InboxItem[],
    mood: value.mood as Mood,
    xp: Math.floor(value.xp),
    petName:
      typeof value.petName === 'string' && value.petName.trim() && value.petName.trim() !== '小章章'
        ? value.petName.trim()
        : defaultFocusFlowState.petName,
    petSetId: isPetSetId(value.petSetId) ? value.petSetId : DEFAULT_PET_SET_ID,
    showFocusReturn: typeof value.showFocusReturn === 'boolean' ? value.showFocusReturn : true,
    hasCompletedPetNaming: value.hasCompletedPetNaming === true,
  };
}

export async function loadFocusFlowState(): Promise<FocusFlowPersistedState> {
  try {
    return normalizeState(await invoke('load_focus_flow_state')) ?? cloneDefaultFocusFlowState();
  } catch {
    return cloneDefaultFocusFlowState();
  }
}

export async function saveFocusFlowState(
  state: Pick<
    FocusFlowPersistedState,
    'tasks' | 'inboxItems' | 'mood' | 'xp' | 'petName' | 'petSetId' | 'showFocusReturn' | 'hasCompletedPetNaming'
  >,
) {
  try {
    await invoke('save_focus_flow_state', {
      payload: {
        version: 4,
        tasks: state.tasks,
        inboxItems: state.inboxItems,
        mood: state.mood,
        xp: Math.max(0, Math.floor(state.xp)),
        petName: state.petName.trim() || defaultFocusFlowState.petName,
        petSetId: isPetSetId(state.petSetId) ? state.petSetId : DEFAULT_PET_SET_ID,
        showFocusReturn: state.showFocusReturn,
        hasCompletedPetNaming: state.hasCompletedPetNaming,
      } satisfies FocusFlowPersistedState,
    });
  } catch {
    // Persistence remains best-effort so UI actions are not blocked by disk errors.
  }
}
