import { CSSProperties, useEffect, useState } from 'react';
import { getPetLevelConfig, getPetSetConfig, type PetSetConfig, type PetSetId } from './petConfig';
import styles from './PetRenderer.module.css';

interface PetRendererProps {
  level: number;
  petSets: PetSetConfig[];
  petSetId?: PetSetId;
  size?: 'normal' | 'focus';
  label?: string;
}

function assetUrl(petSet: PetSetConfig, path: string) {
  if (!petSet.assetRoot) {
    return `asset://localhost/${encodeURI(path)}`;
  }

  return `${import.meta.env.BASE_URL}${petSet.assetRoot}/${path}`;
}

export function PetRenderer({ level, petSets, petSetId, size = 'normal', label }: PetRendererProps) {
  const [hasImageError, setHasImageError] = useState(false);
  const [frameIndex, setFrameIndex] = useState(0);
  const petSet = getPetSetConfig(petSets, petSetId);
  const petLevel = getPetLevelConfig(petSet, level);
  const animation = petLevel.animation;
  const activeFrameIndex = frameIndex % animation.frames.length;

  useEffect(() => {
    setHasImageError(false);
    setFrameIndex(0);
  }, [petSet.id, level]);

  useEffect(() => {
    const intervalMs = animation.durationMs / animation.frames.length;
    const intervalId = window.setInterval(() => {
      setFrameIndex((current) => (current + 1) % animation.frames.length);
    }, intervalMs);

    return () => window.clearInterval(intervalId);
  }, [animation.durationMs, animation.frames.length, petSet.id, level]);

  if (hasImageError) {
    return <div className={`${styles.petRenderer} ${styles[size]}`} aria-label={label} role={label ? 'img' : undefined}>
      <div className={styles.fallback} />
    </div>;
  }

  return (
    <div className={`${styles.petRenderer} ${styles[size]}`} aria-label={label} role={label ? 'img' : undefined}>
      {animation.frames.map((frame, index) => (
        <img
          alt=""
          aria-hidden="true"
          className={`${styles.frame} ${index === activeFrameIndex ? styles.activeFrame : ''}`}
          draggable={false}
          key={frame}
          src={assetUrl(petSet, frame)}
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
          src={assetUrl(petSet, item.src)}
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
