export type PetExpression = 'normal' | 'happy' | 'annoyed';

export interface PetAnimationConfig {
  frames: string[];
  durationMs: number;
  loop: boolean;
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
  name: string;
  minXp: number;
  frameSize: {
    width: number;
    height: number;
  };
  animations: Record<PetExpression, PetAnimationConfig>;
  items: PetItemConfig[];
}

interface PetAssetConfig {
  version: 1;
  assetRoot: string;
  levels: PetLevelConfig[];
}

export const petAssetConfig: PetAssetConfig = {
  version: 1,
  assetRoot: 'pet-assets',
  levels: [
    {
      level: 1,
      name: '小鲨',
      minXp: 0,
      frameSize: { width: 256, height: 256 },
      animations: {
        normal: { frames: ['lv1/normal-0.png', 'lv1/normal-1.png', 'lv1/normal-2.png'], durationMs: 2200, loop: true },
        happy: { frames: ['lv1/happy-0.png', 'lv1/happy-1.png', 'lv1/happy-2.png', 'lv1/happy-3.png'], durationMs: 1700, loop: true },
        annoyed: { frames: ['lv1/annoyed-0.png', 'lv1/annoyed-1.png', 'lv1/annoyed-2.png', 'lv1/annoyed-3.png'], durationMs: 1800, loop: true },
      },
      items: [],
    },
    {
      level: 2,
      name: '蓝鲨',
      minXp: 50,
      frameSize: { width: 256, height: 256 },
      animations: {
        normal: { frames: ['lv2/normal-0.png', 'lv2/normal-1.png', 'lv2/normal-2.png'], durationMs: 2200, loop: true },
        happy: { frames: ['lv2/happy-0.png', 'lv2/happy-1.png', 'lv2/happy-2.png', 'lv2/happy-3.png'], durationMs: 1700, loop: true },
        annoyed: { frames: ['lv2/annoyed-0.png', 'lv2/annoyed-1.png', 'lv2/annoyed-2.png', 'lv2/annoyed-3.png'], durationMs: 1800, loop: true },
      },
      items: [
        { id: 'bubble', src: 'items/bubble-lv2.png', anchor: 'top-center', offset: { x: 10, y: -18 }, zIndex: 2 },
      ],
    },
    {
      level: 3,
      name: '鲨王',
      minXp: 350,
      frameSize: { width: 256, height: 256 },
      animations: {
        normal: { frames: ['lv3/normal-0.png', 'lv3/normal-1.png', 'lv3/normal-2.png'], durationMs: 2200, loop: true },
        happy: { frames: ['lv3/happy-0.png', 'lv3/happy-1.png', 'lv3/happy-2.png', 'lv3/happy-3.png'], durationMs: 1700, loop: true },
        annoyed: { frames: ['lv3/annoyed-0.png', 'lv3/annoyed-1.png', 'lv3/annoyed-2.png', 'lv3/annoyed-3.png'], durationMs: 1800, loop: true },
      },
      items: [
        { id: 'crown', src: 'items/crown-lv3.png', anchor: 'top-center', offset: { x: -6, y: -24 }, rotationDeg: -12, zIndex: 2 },
      ],
    },
  ],
};

export function getPetLevelConfig(level: number) {
  return petAssetConfig.levels.find((item) => item.level === level) ?? petAssetConfig.levels[0];
}
