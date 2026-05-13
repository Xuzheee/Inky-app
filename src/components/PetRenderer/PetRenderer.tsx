import { CSSProperties, useState } from 'react';
import { getPetLevelConfig, type PetExpression } from './petConfig';
import styles from './PetRenderer.module.css';

interface PetRendererProps {
  level: number;
  expression: PetExpression;
  size?: 'normal' | 'focus';
  label?: string;
}

function assetUrl(path: string) {
  return `${import.meta.env.BASE_URL}pet-assets/${path}`;
}

export function PetRenderer({ level, expression, size = 'normal', label }: PetRendererProps) {
  const [hasImageError, setHasImageError] = useState(false);
  const petLevel = getPetLevelConfig(level);
  const animation = petLevel.animations[expression];
  const frameCountClass = animation.frames.length === 4 ? styles.frameCount4 : styles.frameCount3;

  if (hasImageError) {
    return <div className={`${styles.petRenderer} ${styles[size]}`} aria-label={label} role={label ? 'img' : undefined}>
      <div className={styles.fallback} />
    </div>;
  }

  return (
    <div className={`${styles.petRenderer} ${styles[size]} ${frameCountClass}`} aria-label={label} role={label ? 'img' : undefined}>
      {animation.frames.map((frame, index) => (
        <img
          alt=""
          aria-hidden="true"
          className={styles.frame}
          draggable={false}
          key={frame}
          src={assetUrl(frame)}
          style={{
            '--pet-duration': `${animation.durationMs}ms`,
            '--pet-frame-count': animation.frames.length,
            '--pet-frame-index': index,
          } as CSSProperties}
          onError={() => setHasImageError(true)}
        />
      ))}
      {petLevel.items.map((item) => (
        <img
          alt=""
          aria-hidden="true"
          className={styles.item}
          draggable={false}
          key={item.id}
          src={assetUrl(item.src)}
          style={{
            '--pet-item-x': `${item.offset.x}px`,
            '--pet-item-y': `${item.offset.y}px`,
            '--pet-item-rotation': `${item.rotationDeg ?? 0}deg`,
            '--pet-item-z': item.zIndex,
          } as CSSProperties}
          onError={() => setHasImageError(true)}
        />
      ))}
    </div>
  );
}
