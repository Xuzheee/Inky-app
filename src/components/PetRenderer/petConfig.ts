export type PetSetId = string;

export interface PetAnimationConfig {
  frames: string[];
  durationMs: number;
}

export interface PetItemConfig {
  id: string;
  src: string;
  anchor: 'top-center';
  offset: {
    x: number;
    y: number;
  };
  rotationDeg?: number;
  zIndex: number;
}

export interface PetLevelConfig {
  level: 1 | 2 | 3;
  minXp: number;
  animation: PetAnimationConfig;
  items: PetItemConfig[];
}

export interface PetSetConfig {
  id: PetSetId;
  name: string;
  description: string;
  assetRoot?: string;
  levels: PetLevelConfig[];
}

export interface LocalPetPackLevel {
  level: 1 | 2 | 3;
  frames: string[];
  durationMs: number;
}

export interface LocalPetPack {
  id: string;
  name: string;
  description: string;
  levels: LocalPetPackLevel[];
}

export const DEFAULT_PET_SET_ID = 'builtin:inky';

const inkyLevels: PetLevelConfig[] = [
  {
    level: 1,
    minXp: 0,
    animation: { frames: ['lv1/normal-0.png', 'lv1/normal-1.png', 'lv1/normal-2.png'], durationMs: 2200 },
    items: [],
  },
  {
    level: 2,
    minXp: 50,
    animation: { frames: ['lv2/normal-0.png', 'lv2/normal-1.png', 'lv2/normal-2.png'], durationMs: 2200 },
    items: [
      { id: 'bubble', src: 'items/bubble-lv2.png', anchor: 'top-center', offset: { x: 10, y: -18 }, zIndex: 2 },
    ],
  },
  {
    level: 3,
    minXp: 350,
    animation: { frames: ['lv3/normal-0.png', 'lv3/normal-1.png', 'lv3/normal-2.png'], durationMs: 2200 },
    items: [
      { id: 'crown', src: 'items/crown-lv3.png', anchor: 'top-center', offset: { x: -6, y: -24 }, rotationDeg: -12, zIndex: 2 },
    ],
  },
];

export const builtInPetSets: PetSetConfig[] = [
  {
    id: DEFAULT_PET_SET_ID,
    name: 'Inky',
    description: '内置默认宠物。',
    assetRoot: 'pet-assets',
    levels: inkyLevels,
  },
];

function normalizeLevel(level: number): 1 | 2 | 3 {
  return level === 2 || level === 3 ? level : 1;
}

export function isPetSetId(value: unknown): value is PetSetId {
  return typeof value === 'string' && (value === DEFAULT_PET_SET_ID || value.startsWith('local:'));
}

export function localPackToPetSet(pack: LocalPetPack): PetSetConfig {
  return {
    id: pack.id,
    name: pack.name,
    description: pack.description,
    levels: pack.levels.map((level) => ({
      level: normalizeLevel(level.level),
      minXp: level.level === 1 ? 0 : level.level === 2 ? 50 : 350,
      animation: {
        frames: level.frames,
        durationMs: level.durationMs,
      },
      items: [],
    })),
  };
}

export function getPetSetConfig(petSets: PetSetConfig[], petSetId: unknown) {
  return petSets.find((petSet) => petSet.id === petSetId) ?? petSets[0] ?? builtInPetSets[0];
}

export function getPetLevelConfig(petSet: PetSetConfig, level: number) {
  return petSet.levels.find((item) => item.level === level) ?? petSet.levels[0];
}
