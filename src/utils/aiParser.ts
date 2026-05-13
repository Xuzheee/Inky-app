import { invoke } from '@tauri-apps/api/core';
import type { Mood, Task, TaskCategory, TaskPriority } from '../types';

const PARSE_TIMEOUT_MS = 5000;
const MAX_PARSED_TASKS = 5;

const taskCategories = new Set<TaskCategory>(['work', 'study', 'life', 'idea']);
const taskPriorities = new Set<TaskPriority>(['high', 'medium', 'low']);

export interface ParseTaskRequest {
  input: string;
  nowIso: string;
  mood: Mood;
  existingTasks: Array<Pick<Task, 'title' | 'category' | 'priority' | 'due' | 'completed'>>;
}

export interface ParsedTaskDraft {
  title: string;
  category: TaskCategory;
  priority: TaskPriority;
  due: string | null;
  selected: boolean;
}

export interface ParsedTaskResult {
  tasks: ParsedTaskDraft[];
  petComment: string;
  source: 'stub' | 'claude' | 'deepseek';
}

interface RawParsedTaskDraft {
  title?: unknown;
  category?: unknown;
  priority?: unknown;
  due?: unknown;
}

interface RawParsedTaskResult {
  tasks?: unknown;
  petComment?: unknown;
  source?: unknown;
}

function timeoutAfter() {
  return new Promise<null>((resolve) => {
    window.setTimeout(() => resolve(null), PARSE_TIMEOUT_MS);
  });
}

function normalizeDraft(value: unknown): ParsedTaskDraft | null {
  const draft = value as RawParsedTaskDraft;

  if (
    typeof draft.title !== 'string' ||
    !draft.title.trim() ||
    typeof draft.category !== 'string' ||
    !taskCategories.has(draft.category as TaskCategory) ||
    typeof draft.priority !== 'string' ||
    !taskPriorities.has(draft.priority as TaskPriority) ||
    !(typeof draft.due === 'string' || draft.due === null)
  ) {
    return null;
  }

  return {
    title: draft.title.trim(),
    category: draft.category as TaskCategory,
    priority: draft.priority as TaskPriority,
    due: draft.due,
    selected: true,
  };
}

function normalizeResult(value: unknown): ParsedTaskResult | null {
  const result = value as RawParsedTaskResult;

  if (
    !result ||
    !Array.isArray(result.tasks) ||
    typeof result.petComment !== 'string' ||
    (result.source !== 'stub' && result.source !== 'claude' && result.source !== 'deepseek')
  ) {
    return null;
  }

  const tasks = result.tasks.slice(0, MAX_PARSED_TASKS).map(normalizeDraft);

  if (tasks.some((task) => task === null)) {
    return null;
  }

  const parsedTasks = tasks as ParsedTaskDraft[];

  if (parsedTasks.length === 0) {
    return null;
  }

  return {
    tasks: parsedTasks,
    petComment: result.petComment,
    source: result.source,
  };
}

async function invokeParser(request: ParseTaskRequest) {
  return normalizeResult(await invoke('parse_task_with_ai', { request }));
}

export async function parseTaskWithAi(request: ParseTaskRequest): Promise<ParsedTaskResult | null> {
  try {
    return await Promise.race([invokeParser(request), timeoutAfter()]);
  } catch {
    return null;
  }
}
