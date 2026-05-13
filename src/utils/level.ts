import type { LevelConfig, LevelProgress } from '../types';

export const LEVELS: LevelConfig[] = [
  { level: 1, title: '小鲨', requiredXp: 0 },
  { level: 2, title: '蓝鲨', requiredXp: 50 },
  { level: 3, title: '鲨王', requiredXp: 350 },
];

export function getLevelProgress(xp: number): LevelProgress {
  const current = LEVELS.reduce((matched, level) => (xp >= level.requiredXp ? level : matched), LEVELS[0]);
  const next = LEVELS.find((level) => level.requiredXp > xp) ?? null;

  if (!next) {
    return {
      current,
      next,
      progressPercent: 100,
      xpIntoLevel: xp - current.requiredXp,
      xpForNextLevel: null,
    };
  }

  const xpIntoLevel = xp - current.requiredXp;
  const xpForNextLevel = next.requiredXp - current.requiredXp;

  return {
    current,
    next,
    progressPercent: Math.round((xpIntoLevel / xpForNextLevel) * 100),
    xpIntoLevel,
    xpForNextLevel,
  };
}
