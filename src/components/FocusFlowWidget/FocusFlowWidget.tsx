import { invoke } from '@tauri-apps/api/core';
import { FormEvent, useEffect, useMemo, useRef, useState } from 'react';
import type { Mood, OverlayState, TaskCategory, TaskPriority, ViewState } from '../../types';
import { PetRenderer } from '../PetRenderer/PetRenderer';
import type { PetExpression } from '../PetRenderer/petConfig';
import { parseTaskWithAi, type ParsedTaskDraft } from '../../utils/aiParser';
import {
  clearAiConfig,
  emptyAiConfigStatus,
  getAppLocalDataDir,
  loadAiConfigStatus,
  saveInviteAiConfig,
  savePersonalAiConfig,
  type AiConfigMode,
} from '../../utils/aiSettings';
import { getLevelProgress } from '../../utils/level';
import { cloneDefaultFocusFlowState, defaultFocusFlowState, loadFocusFlowState, saveFocusFlowState } from '../../utils/focusFlowPersistence';
import styles from './FocusFlowWidget.module.css';

const categoryLabel: Record<TaskCategory, string> = {
  work: '工作',
  study: '学习',
  life: '生活',
  idea: '想法',
};

const categoryIcon: Record<TaskCategory, string> = {
  work: '◇',
  study: '✦',
  life: '●',
  idea: '✧',
};

const moodEmoji: Record<Mood, string> = {
  好: '😊',
  一般: '😐',
  烦: '😤',
};

const priorityLabel: Record<TaskPriority, string> = {
  high: '高优先级',
  medium: '中优先级',
  low: '低优先级',
};

const moodPetExpression: Record<Mood, PetExpression> = {
  好: 'happy',
  一般: 'normal',
  烦: 'annoyed',
};

const POMODORO_DURATION_SECONDS = 25 * 60;
const TOAST_AUTO_DISMISS_MS = 5_000;
const DRIFT_THRESHOLD_MS = 15 * 60 * 1000;

type ReminderType = 'cheer' | 'break' | 'drift';

type Reminder = {
  type: ReminderType;
  title: string;
  message: string;
};

function formatTime(seconds: number) {
  const minutes = Math.floor(seconds / 60).toString().padStart(2, '0');
  const rest = (seconds % 60).toString().padStart(2, '0');
  return `${minutes}:${rest}`;
}

function callDesktopCommand(command: string, args?: Record<string, unknown>) {
  invoke(command, args).catch(() => undefined);
}

async function getSystemIdleMs() {
  try {
    const idleMs = await invoke<number>('get_system_idle_ms');

    if (!Number.isFinite(idleMs) || idleMs < 0) {
      return null;
    }

    return idleMs;
  } catch {
    return null;
  }
}

function canShowReminder(type: ReminderType, mood: Mood) {
  return type !== 'cheer' || mood !== '烦';
}

export function FocusFlowWidget() {
  const [initialFocusFlowState] = useState(cloneDefaultFocusFlowState);
  const [tasks, setTasks] = useState(initialFocusFlowState.tasks);
  const [mood, setMood] = useState<Mood>(initialFocusFlowState.mood);
  const [xp, setXp] = useState(initialFocusFlowState.xp);
  const [petName, setPetName] = useState(initialFocusFlowState.petName);
  const [isPersistenceLoaded, setIsPersistenceLoaded] = useState(false);
  const [input, setInput] = useState('');
  const [overlay, setOverlay] = useState<OverlayState>('none');
  const [view, setView] = useState<ViewState>('main');
  const [isMiniMode, setIsMiniMode] = useState(false);
  const [focusedTaskId, setFocusedTaskId] = useState<string | null>(null);
  const [parsedTasks, setParsedTasks] = useState<ParsedTaskDraft[]>([]);
  const [petComment, setPetComment] = useState('');
  const [isParsingTask, setIsParsingTask] = useState(false);
  const [timeLeft, setTimeLeft] = useState(POMODORO_DURATION_SECONDS);
  const [pomodoroEndsAt, setPomodoroEndsAt] = useState<number | null>(null);
  const parseRequestId = useRef(0);
  const pomodoroAwardedForCycle = useRef(false);
  const [toast, setToast] = useState<Reminder | null>(null);
  const [miniHint, setMiniHint] = useState<Reminder | null>(null);
  const [aiConfigStatus, setAiConfigStatus] = useState(emptyAiConfigStatus);
  const [aiConfigMode, setAiConfigMode] = useState<AiConfigMode>('personalKey');
  const [personalKeyInput, setPersonalKeyInput] = useState('');
  const [inviteCodeInput, setInviteCodeInput] = useState('');
  const [proxyUrlInput, setProxyUrlInput] = useState('');
  const [appLocalDataDir, setAppLocalDataDir] = useState('');
  const [settingsMessage, setSettingsMessage] = useState('');
  const driftShownForIdleStretch = useRef(false);
  const cheerShown = useRef(false);

  const level = useMemo(() => getLevelProgress(xp), [xp]);
  const focusedTask = tasks.find((task) => task.id === focusedTaskId) ?? null;
  const sortedTasks = [...tasks].sort((a, b) => (a.completed === b.completed ? 0 : a.completed ? 1 : -1));
  const remainingTasks = tasks.filter((task) => !task.completed).length;
  const completedTasks = tasks.length - remainingTasks;
  const isAiParsing = input.trim().length > 8;
  const petExpression = moodPetExpression[mood];

  useEffect(() => {
    let cancelled = false;

    async function loadPersistedState() {
      const persistedState = await loadFocusFlowState();

      if (cancelled) {
        return;
      }

      setTasks(persistedState.tasks);
      setMood(persistedState.mood);
      setXp(persistedState.xp);
      setPetName(persistedState.petName);
      setIsPersistenceLoaded(true);
    }

    void loadPersistedState();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!isPersistenceLoaded) {
      return;
    }

    void saveFocusFlowState({ tasks, mood, xp, petName });
  }, [isPersistenceLoaded, tasks, mood, xp, petName]);

  useEffect(() => {
    let cancelled = false;

    async function loadSettings() {
      try {
        const [status, localDataDir] = await Promise.all([loadAiConfigStatus(), getAppLocalDataDir()]);

        if (cancelled) {
          return;
        }

        setAiConfigStatus(status);
        setAiConfigMode(status.mode === 'invite' ? 'invite' : 'personalKey');
        setProxyUrlInput(status.proxyUrl ?? '');
        setAppLocalDataDir(localDataDir);
      } catch {
        if (!cancelled) {
          setSettingsMessage('设置状态暂时不可用。');
        }
      }
    }

    void loadSettings();

    return () => {
      cancelled = true;
    };
  }, []);

  useEffect(() => {
    if (!toast) {
      return;
    }

    const timeoutId = window.setTimeout(() => setToast(null), TOAST_AUTO_DISMISS_MS);

    return () => window.clearTimeout(timeoutId);
  }, [toast]);

  useEffect(() => {
    if (!miniHint) {
      return;
    }

    const timeoutId = window.setTimeout(() => setMiniHint(null), TOAST_AUTO_DISMISS_MS);

    return () => window.clearTimeout(timeoutId);
  }, [miniHint]);

  useEffect(() => {
    if (cheerShown.current || view !== 'main' || isMiniMode || overlay !== 'none') {
      return;
    }

    cheerShown.current = true;

    if (mood === '好') {
      showToast('cheer', '准备开始', 'pet 在这里陪你。先写下一件最小的事就好。');
      return;
    }

    if (mood === '一般') {
      showToast('cheer', '慢慢来', '今天不用一下子做很多，先抓住一个小任务。');
    }
  }, [view, isMiniMode, overlay, mood]);

  useEffect(() => {
    if (view !== 'focus' || !focusedTask || overlay !== 'none' || pomodoroEndsAt === null) {
      return;
    }

    const endsAt = pomodoroEndsAt;

    function syncTimeLeft() {
      setTimeLeft(Math.max(0, Math.ceil((endsAt - Date.now()) / 1000)));
    }

    syncTimeLeft();
    const intervalId = window.setInterval(syncTimeLeft, 1000);

    return () => window.clearInterval(intervalId);
  }, [view, focusedTask, overlay, pomodoroEndsAt]);

  useEffect(() => {
    if (view !== 'focus' || !focusedTask || overlay !== 'none') {
      return;
    }

    const focusedTaskTitle = focusedTask.title;

    async function checkDrift() {
      const idleMs = await getSystemIdleMs();

      if (idleMs === null) {
        return;
      }

      if (idleMs < DRIFT_THRESHOLD_MS) {
        driftShownForIdleStretch.current = false;
        return;
      }

      if (driftShownForIdleStretch.current) {
        return;
      }

      driftShownForIdleStretch.current = true;
      const title = '回到任务';
      const message = `刚才可能飘走了一下。现在只要回到「${focusedTaskTitle}」这一件事。`;

      if (isMiniMode) {
        showMiniHint('drift', title, message);
        return;
      }

      showToast('drift', title, message);
    }

    void checkDrift();
    const intervalId = window.setInterval(() => void checkDrift(), 15_000);

    return () => window.clearInterval(intervalId);
  }, [view, focusedTask, overlay, isMiniMode]);

  useEffect(() => {
    if (view !== 'focus' || !focusedTask || timeLeft !== 0 || pomodoroAwardedForCycle.current) {
      return;
    }

    pomodoroAwardedForCycle.current = true;
    setPomodoroEndsAt(null);
    setTasks((current) =>
      current.map((task) =>
        task.id === focusedTask.id ? { ...task, completedPomodoros: task.completedPomodoros + 1 } : task,
      ),
    );
    setXp((currentXp) => currentXp + 5);
    if (isMiniMode) {
      exitMiniMode();
    }
    showToast('break', '番茄完成', `「${focusedTask.title}」已经完成一个专注时段，休息一下再决定下一步。`);
    setOverlay('break');
  }, [view, focusedTask, timeLeft, isMiniMode]);

  function dismissToast() {
    setToast(null);
  }

  function dismissMiniHint() {
    setMiniHint(null);
  }

  function closeToast() {
    recordInteraction();
    dismissToast();
  }

  function closeMiniHint() {
    recordInteraction();
    dismissMiniHint();
  }

  function showToast(type: ReminderType, title: string, message: string) {
    if (!canShowReminder(type, mood)) {
      return;
    }

    setMiniHint(null);
    setToast({ type, title, message });
  }

  function showMiniHint(type: ReminderType, title: string, message: string) {
    if (!canShowReminder(type, mood)) {
      return;
    }

    setToast(null);
    setMiniHint({ type, title, message });
  }

  function recordInteraction() {
    dismissMiniHint();
  }

  function updatePetName(value: string) {
    setPetName(value.trim() || defaultFocusFlowState.petName);
  }

  function startDrag() {
    callDesktopCommand('start_window_drag');
  }

  function enterMiniMode() {
    recordInteraction();
    setIsMiniMode(true);
    callDesktopCommand('set_mini_mode', { mini: true });
  }

  function exitMiniMode() {
    recordInteraction();
    callDesktopCommand('set_mini_mode', { mini: false });
    window.setTimeout(() => setIsMiniMode(false), 80);
  }

  function aiStatusLabel() {
    if (aiConfigStatus.mode === 'personalKey') {
      return '正在使用个人 DeepSeek API Key';
    }

    if (aiConfigStatus.mode === 'invite') {
      return '正在使用邀请码代理';
    }

    return '尚未配置 AI 解析';
  }

  function openSettings() {
    recordInteraction();
    setSettingsMessage('');
    setOverlay('settings');
  }

  function closeSettings() {
    recordInteraction();
    setOverlay('none');
    setSettingsMessage('');
  }

  async function savePersonalSettings() {
    recordInteraction();
    setSettingsMessage('正在保存个人 Key...');

    try {
      const status = await savePersonalAiConfig(personalKeyInput);
      setAiConfigStatus(status);
      setAiConfigMode('personalKey');
      setPersonalKeyInput('');
      setSettingsMessage('已保存个人 Key。');
    } catch {
      setSettingsMessage('保存失败，请确认 Key 不为空。');
    }
  }

  async function saveInviteSettings() {
    recordInteraction();
    setSettingsMessage('正在保存邀请码...');

    try {
      const status = await saveInviteAiConfig(inviteCodeInput, proxyUrlInput);
      setAiConfigStatus(status);
      setAiConfigMode('invite');
      setInviteCodeInput('');
      setSettingsMessage('已保存邀请码模式。');
    } catch {
      setSettingsMessage('保存失败，请检查邀请码和代理地址。');
    }
  }

  async function clearSettings() {
    recordInteraction();
    setSettingsMessage('正在清除 AI 配置...');

    try {
      const status = await clearAiConfig();
      setAiConfigStatus(status);
      setPersonalKeyInput('');
      setInviteCodeInput('');
      setSettingsMessage('已清除 AI 配置。');
    } catch {
      setSettingsMessage('清除失败，请稍后重试。');
    }
  }

  async function copyLocalDataPath() {
    recordInteraction();

    try {
      await navigator.clipboard.writeText(appLocalDataDir);
      setSettingsMessage('本地数据路径已复制。');
    } catch {
      setSettingsMessage('复制失败，请手动选择路径。');
    }
  }

  function addTask(
    title: string,
    category: TaskCategory = 'idea',
    priority: TaskPriority = 'medium',
    due: string | null = null,
  ) {
    setTasks((current) => [
      { id: crypto.randomUUID(), title, category, priority, completed: false, due, completedPomodoros: 0 },
      ...current,
    ]);
  }

  function resetParsedState() {
    setParsedTasks([]);
    setPetComment('');
  }

  function addInputTask(category: TaskCategory) {
    recordInteraction();
    addTask(input.trim(), category);
    setInput('');
    setOverlay('none');
    resetParsedState();
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    recordInteraction();
    const title = input.trim();

    if (!title || isParsingTask) {
      return;
    }

    resetParsedState();

    if (title.length <= 8) {
      setOverlay('category');
      return;
    }

    const requestId = parseRequestId.current + 1;
    parseRequestId.current = requestId;
    setIsParsingTask(true);

    const result = await parseTaskWithAi({
      input: title,
      nowIso: new Date().toISOString(),
      mood,
      existingTasks: tasks.map(({ title: taskTitle, category, priority, due, completed }) => ({
        title: taskTitle,
        category,
        priority,
        due,
        completed,
      })),
    });

    if (parseRequestId.current !== requestId || input.trim() !== title) {
      setIsParsingTask(false);
      return;
    }

    setIsParsingTask(false);

    if (!result) {
      setOverlay('category');
      return;
    }

    setParsedTasks(result.tasks);
    setPetComment(result.petComment);
    setOverlay('ai-parse');
  }

  function updateParsedTask(index: number, updates: Partial<ParsedTaskDraft>) {
    recordInteraction();
    setParsedTasks((current) => current.map((task, taskIndex) => (taskIndex === index ? { ...task, ...updates } : task)));
  }

  function closeAiSheet() {
    recordInteraction();
    setOverlay('none');
    resetParsedState();
  }

  function acceptParsedTask() {
    recordInteraction();
    const selectedTasks = parsedTasks.filter((task) => task.selected && task.title.trim());

    if (selectedTasks.length === 0) {
      return;
    }

    selectedTasks.forEach((task) => addTask(task.title.trim(), task.category, task.priority, task.due));
    setInput('');
    setOverlay('none');
    resetParsedState();
  }

  function toggleTask(taskId: string) {
    recordInteraction();
    const taskToToggle = tasks.find((task) => task.id === taskId);

    if (!taskToToggle) {
      return;
    }

    const willComplete = !taskToToggle.completed;
    const nextTasks = tasks.map((task) => (task.id === taskId ? { ...task, completed: willComplete } : task));

    setTasks(nextTasks);

    if (willComplete) {
      const allDone = nextTasks.every((task) => task.completed);
      setXp((currentXp) => currentXp + 10 + (allDone ? 20 : 0));
    }
  }

  function removeTask(taskId: string) {
    recordInteraction();
    setTasks((current) => current.filter((task) => task.id !== taskId));
  }

  function startFocus(taskId: string) {
    recordInteraction();
    setFocusedTaskId(taskId);
    setView('focus');
    setOverlay('none');
    setTimeLeft(POMODORO_DURATION_SECONDS);
    setPomodoroEndsAt(Date.now() + POMODORO_DURATION_SECONDS * 1000);
    pomodoroAwardedForCycle.current = false;
    driftShownForIdleStretch.current = false;
  }

  function leaveFocusSession() {
    recordInteraction();
    setOverlay('none');
    setView('main');
    setFocusedTaskId(null);
    setTimeLeft(POMODORO_DURATION_SECONDS);
    setPomodoroEndsAt(null);
    pomodoroAwardedForCycle.current = false;
  }

  function continuePomodoro() {
    recordInteraction();
    if (!focusedTask) {
      return;
    }

    setOverlay('none');
    setTimeLeft(POMODORO_DURATION_SECONDS);
    setPomodoroEndsAt(Date.now() + POMODORO_DURATION_SECONDS * 1000);
    pomodoroAwardedForCycle.current = false;
    driftShownForIdleStretch.current = false;
  }

  function completeFocusedTask() {
    recordInteraction();
    if (!focusedTask || focusedTask.completed) {
      leaveFocusSession();
      return;
    }

    const nextTasks = tasks.map((task) => (task.id === focusedTask.id ? { ...task, completed: true } : task));

    setTasks(nextTasks);
    setXp((currentXp) => currentXp + 10 + (nextTasks.every((task) => task.completed) ? 20 : 0));
    leaveFocusSession();
  }

  if (isMiniMode && overlay === 'none') {
    return (
      <div className={styles.miniShell}>
        <button
          className={styles.miniPetButton}
          type="button"
          aria-label="恢复 Inky"
          onClick={exitMiniMode}
        >
          <PetRenderer level={level.current.level} expression={petExpression} label={`Inky${level.current.title}`} />
        </button>
        {miniHint && (
          <div className={styles.miniHint} role="status">
            <button type="button" aria-label="关闭提醒" onClick={closeMiniHint}>
              ×
            </button>
            <strong>{miniHint.title}</strong>
            <span>{miniHint.message}</span>
          </div>
        )}
        <button className={styles.miniDragHandle} type="button" aria-label="拖动宠物窗口" onMouseDown={startDrag}>
          ⋮⋮
        </button>
      </div>
    );
  }

  if (view === 'focus' && focusedTask) {
    return (
      <section className={`${styles.shell} ${styles.focusView}`}>
        <div className={styles.focusAura} />
        <header className={styles.titlebar} onMouseDown={startDrag}>
          <div className={styles.windowTitleGroup}>
            <div className={styles.windowControls} aria-hidden="true">
              <span className={styles.closeDot} />
              <span className={styles.minDot} />
              <span className={styles.maxDot} />
            </div>
            <span className={styles.brand}>Inky</span>
          </div>
          <div className={styles.titleActions} onMouseDown={(event) => event.stopPropagation()}>
            <button className={styles.miniButton} type="button" onClick={enterMiniMode} aria-label="缩小为宠物模式">
              pet
            </button>
            <button className={styles.ghostButton} type="button" onClick={leaveFocusSession}>
              放弃专注
            </button>
          </div>
        </header>

        <div className={styles.focusContent}>
          <div className={styles.focusPet}>
            <PetRenderer level={level.current.level} expression="normal" size="focus" label={`Inky${level.current.title}`} />
          </div>
          <p className={styles.focusLabel}>现在专注于</p>
          <h1 className={styles.focusTitle}>📝 {focusedTask.title}</h1>
          <p className={styles.focusPomodoroCount}>已完成 {focusedTask.completedPomodoros} 个番茄</p>
          <div className={styles.timer}>{formatTime(timeLeft)}</div>
          <button className={styles.primaryButton} type="button" onClick={leaveFocusSession}>
            暂停并返回
          </button>
        </div>

        <footer className={styles.statsBar}>
          <span className={styles.timerStat}>{formatTime(timeLeft)}</span>
          <div className={styles.statsGroup}>
            <span><strong>{completedTasks}</strong><em>完成</em></span>
            <span><strong>{xp}</strong><em>XP</em></span>
          </div>
        </footer>

        {overlay === 'break' && (
          <div className={styles.scrim}>
            <div
              aria-labelledby="break-overlay-title"
              aria-modal="true"
              className={styles.pomodoroDialog}
              role="dialog"
            >
              <div className={styles.celebration}>🎉</div>
              <h2 id="break-overlay-title">番茄钟结束！</h2>
              <p>针对「{focusedTask.title}」的专注时段已完成。</p>
              <div className={styles.pomodoroActions}>
                <button type="button" onClick={completeFocusedTask}>完成任务，再 +10 XP</button>
                <button type="button" onClick={continuePomodoro}>还没完，继续下个番茄钟</button>
                <button type="button" onClick={leaveFocusSession}>先放着，休息一下</button>
              </div>
            </div>
          </div>
        )}

        {toast && (
          <div className={styles.toast} role="status">
            <div className={styles.toastHeader}>
              <span>● {toast.title}</span>
              <button type="button" aria-label="关闭提醒" onClick={closeToast}>×</button>
            </div>
            <p>{toast.message}</p>
          </div>
        )}
      </section>
    );
  }

  return (
    <section className={styles.widgetWrap}>
      <div className={styles.shell}>
        <header className={styles.titlebar} onMouseDown={startDrag}>
          <div className={styles.windowTitleGroup}>
            <div className={styles.windowControls} aria-hidden="true">
              <span className={styles.closeDot} />
              <span className={styles.minDot} />
              <span className={styles.maxDot} />
            </div>
            <span className={styles.brand}>Inky</span>
          </div>
          <div className={styles.titleActions} onMouseDown={(event) => event.stopPropagation()}>
            <button className={styles.iconButton} type="button" onClick={openSettings} aria-label="打开设置">
              ⚙
            </button>
            <button className={styles.miniButton} type="button" onClick={enterMiniMode} aria-label="缩小为宠物模式">
              pet
            </button>
            <div className={styles.shortcut}>Alt F</div>
          </div>
        </header>

        <main className={styles.widgetBody}>
          <section className={styles.petSection}>
            <div className={styles.petAvatar}>
              <PetRenderer level={level.current.level} expression={petExpression} label={`Inky${level.current.title}`} />
              <span className={styles.petLevel}>{level.current.level}</span>
            </div>
            <div className={styles.petInfo}>
              <div className={styles.petNameRow}>
                <input
                  className={styles.petNameInput}
                  value={petName}
                  onChange={(event) => {
                    recordInteraction();
                    setPetName(event.target.value);
                  }}
                  onBlur={(event) => updatePetName(event.target.value)}
                  aria-label="宠物名字"
                  maxLength={8}
                />
                <span>Lv.{level.current.level} {level.current.title}</span>
              </div>
              <p className={styles.petSpeech}>“准备好开始新的任务了吗？”</p>
              <div className={styles.progressBlock}>
                <div className={styles.progressHeader}>
                  <span>{xp} XP</span>
                  <span>{level.next ? `→ Lv.${level.next.level} 需 ${level.next.requiredXp}` : '满级'}</span>
                </div>
                <div className={styles.progressTrack}>
                  <div className={styles.progressFill} style={{ width: `${level.progressPercent}%` }} />
                </div>
              </div>
            </div>
          </section>

          <section className={styles.moodBand}>
            <span>当前状态</span>
            <div className={styles.moodBar}>
              {(['好', '一般', '烦'] as Mood[]).map((item) => (
                <button
                  aria-pressed={item === mood}
                  className={item === mood ? styles.activeMood : undefined}
                  key={item}
                  type="button"
                  onClick={() => {
                    recordInteraction();
                    setMood(item);
                  }}
                >
                  {moodEmoji[item]} {item}
                </button>
              ))}
            </div>
          </section>

          <form className={`${styles.captureForm} ${isAiParsing ? styles.aiReady : ''}`} onSubmit={handleSubmit}>
            <span className={styles.captureGlyph} aria-hidden="true">✦</span>
            <input
              value={input}
              onChange={(event) => {
                recordInteraction();
                setInput(event.target.value);
              }}
              placeholder="随时记下想法..."
              aria-label="快速记录任务"
              disabled={isParsingTask}
            />
            {input.trim() && (
              <button type="submit" disabled={isParsingTask}>
                {isParsingTask ? '解析中' : isAiParsing ? 'AI 解析' : 'Enter ↵'}
              </button>
            )}
          </form>

          <section className={styles.taskSection}>
            <div className={styles.taskHeader}>
              <span>今日任务 ({remainingTasks})</span>
            </div>

            <div className={styles.taskList}>
              {sortedTasks.map((task) => (
                <article className={`${styles.taskItem} ${task.completed ? styles.completedTask : ''}`} key={task.id}>
                  <button
                    aria-label={task.completed ? '标记为未完成' : '标记为完成'}
                    className={styles.checkButton}
                    type="button"
                    onClick={() => toggleTask(task.id)}
                  >
                    {task.completed ? '✓' : ''}
                  </button>
                  <div className={styles.taskBody}>
                    <strong title={task.title}>{task.title}</strong>
                    <span className={`${styles.categoryTag} ${styles[task.category]}`}>
                      {categoryIcon[task.category]} {categoryLabel[task.category]}
                    </span>
                    <span className={styles.pomodoroTag}>番茄 {task.completedPomodoros}</span>
                  </div>
                  {!task.completed && (
                    <div className={styles.taskActions}>
                      <button type="button" onClick={() => startFocus(task.id)} aria-label="开始专注">
                        ▶
                      </button>
                      <button type="button" onClick={() => removeTask(task.id)} aria-label="删除任务">
                        ×
                      </button>
                    </div>
                  )}
                </article>
              ))}
            </div>
          </section>
        </main>

        <footer className={styles.statsBar}>
          <span className={styles.timerStat}>25:00</span>
          <div className={styles.statsGroup}>
            <span><strong>{completedTasks}</strong><em>完成</em></span>
            <span><strong>3 🔥</strong><em>连续</em></span>
            <span><strong>{xp}</strong><em>XP</em></span>
          </div>
        </footer>

        {overlay === 'category' && (
          <div
            aria-labelledby="category-overlay-title"
            aria-modal="true"
            className={styles.categoryOverlay}
            role="dialog"
          >
            <h2 id="category-overlay-title">选择分类</h2>
            <div className={styles.categoryActions}>
              {(['work', 'study', 'life', 'idea'] as TaskCategory[]).map((category) => (
                <button className={styles[category]} type="button" key={category} onClick={() => addInputTask(category)}>
                  {categoryLabel[category]}
                </button>
              ))}
            </div>
          </div>
        )}

        {overlay === 'ai-parse' && (
          <div className={styles.scrim}>
            <div
              aria-labelledby="ai-overlay-title"
              aria-modal="true"
              className={styles.aiSheet}
              role="dialog"
            >
              <button className={styles.sheetClose} type="button" onClick={closeAiSheet} aria-label="关闭解析弹窗">
                ×
              </button>
              <p className={styles.sheetTitle} id="ai-overlay-title">✦ AI 解析成功</p>
              <div className={styles.draftList}>
                {parsedTasks.map((task, index) => (
                  <label className={styles.draftItem} key={`${task.category}-${index}`}>
                    <input
                      type="checkbox"
                      checked={task.selected}
                      onChange={(event) => updateParsedTask(index, { selected: event.target.checked })}
                      aria-label="选择解析任务"
                    />
                    <span className={styles.draftBody}>
                      <input
                        value={task.title}
                        onChange={(event) => updateParsedTask(index, { title: event.target.value })}
                        aria-label="编辑解析后的任务名"
                      />
                      <span className={styles.sheetTags}>
                        <span>{priorityLabel[task.priority]}</span>
                        <span>{categoryIcon[task.category]} {categoryLabel[task.category]}</span>
                        {task.due && <span>{task.due}</span>}
                      </span>
                    </span>
                  </label>
                ))}
              </div>
              <p className={styles.petComment}>🐙 “{petComment}”</p>
              <button
                className={styles.confirmButton}
                type="button"
                onClick={acceptParsedTask}
                disabled={!parsedTasks.some((task) => task.selected && task.title.trim())}
              >
                确认添加
              </button>
            </div>
          </div>
        )}

        {overlay === 'settings' && (
          <div className={styles.scrim}>
            <div
              aria-labelledby="settings-overlay-title"
              aria-modal="true"
              className={styles.settingsPanel}
              role="dialog"
            >
              <button className={styles.sheetClose} type="button" onClick={closeSettings} aria-label="关闭设置">
                ×
              </button>
              <p className={styles.sheetTitle} id="settings-overlay-title">设置</p>

              <section className={styles.settingsSection}>
                <h2>AI 设置</h2>
                <p>{aiStatusLabel()}</p>
                <div className={styles.modeSwitch}>
                  <button
                    className={aiConfigMode === 'personalKey' ? styles.activeMode : undefined}
                    type="button"
                    onClick={() => setAiConfigMode('personalKey')}
                  >
                    个人 Key
                  </button>
                  <button
                    className={aiConfigMode === 'invite' ? styles.activeMode : undefined}
                    type="button"
                    onClick={() => setAiConfigMode('invite')}
                  >
                    邀请码
                  </button>
                </div>

                {aiConfigMode === 'personalKey' ? (
                  <div className={styles.settingsField}>
                    <label htmlFor="personal-key-input">DeepSeek API Key</label>
                    <input
                      id="personal-key-input"
                      type="password"
                      value={personalKeyInput}
                      onChange={(event) => setPersonalKeyInput(event.target.value)}
                      placeholder={aiConfigStatus.hasPersonalKey ? '已保存，输入新 Key 可替换' : 'sk-...'}
                    />
                    <p>个人 Key 只保存在这台设备的本地数据目录。</p>
                    <button type="button" onClick={savePersonalSettings}>保存个人 Key</button>
                  </div>
                ) : (
                  <div className={styles.settingsField}>
                    <label htmlFor="invite-code-input">邀请码</label>
                    <input
                      id="invite-code-input"
                      value={inviteCodeInput}
                      onChange={(event) => setInviteCodeInput(event.target.value)}
                      placeholder={aiConfigStatus.hasInviteCode ? '已保存，输入新邀请码可替换' : 'friend-001'}
                    />
                    <label htmlFor="proxy-url-input">代理地址</label>
                    <input
                      id="proxy-url-input"
                      value={proxyUrlInput}
                      onChange={(event) => setProxyUrlInput(event.target.value)}
                      placeholder="https://your-app.vercel.app/api/parse-task"
                    />
                    <p>邀请码模式会把任务文本发送到 Inky 代理服务解析，不会暴露内置 API Key。</p>
                    <button type="button" onClick={saveInviteSettings}>保存邀请码模式</button>
                  </div>
                )}

                <button className={styles.dangerButton} type="button" onClick={clearSettings}>清除当前 AI 配置</button>
              </section>

              <section className={styles.settingsSection}>
                <h2>本地数据</h2>
                <p>SQLite 状态和 AI 配置都存放在本机。</p>
                <div className={styles.pathRow}>
                  <code>{appLocalDataDir || '路径暂不可用'}</code>
                  <button type="button" onClick={copyLocalDataPath} disabled={!appLocalDataDir}>复制</button>
                </div>
              </section>

              <section className={styles.settingsSection}>
                <h2>窗口与快捷键</h2>
                <p>当前全局快捷键：Alt + F，用于显示或隐藏 Inky。</p>
              </section>

              <section className={styles.settingsSection}>
                <h2>关于</h2>
                <p>Inky 会把任务数据保存在本地；邀请码模式会把解析文本和任务上下文发送到代理服务。</p>
              </section>

              {settingsMessage && <p className={styles.settingsMessage}>{settingsMessage}</p>}
            </div>
          </div>
        )}
      </div>

      {toast && (
        <div className={styles.toast} role="status">
          <div className={styles.toastHeader}>
            <span>● {toast.title}</span>
            <button type="button" aria-label="关闭提醒" onClick={closeToast}>×</button>
          </div>
          <p>{toast.message}</p>
        </div>
      )}
    </section>
  );
}
