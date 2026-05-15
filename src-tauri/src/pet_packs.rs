use std::{fs, path::{Path, PathBuf}, process::Command};

use serde::Serialize;
use tauri::{AppHandle, Manager};

const PET_PACKS_DIR: &str = "pet-packs";
const LEVELS: [u8; 3] = [1, 2, 3];
const MIN_FRAMES_PER_LEVEL: usize = 2;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetPackDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub levels: Vec<PetPackLevelDto>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PetPackLevelDto {
    pub level: u8,
    pub frames: Vec<String>,
    pub duration_ms: u64,
}

fn pet_packs_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let path = app
        .path()
        .app_data_dir()
        .map_err(|error| error.to_string())?
        .join(PET_PACKS_DIR);

    fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    Ok(path)
}

fn local_asset_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn frame_index(file_name: &str) -> Option<usize> {
    let stem = file_name.strip_suffix(".png")?;
    let index = stem
        .strip_prefix("frame-")
        .or_else(|| stem.strip_prefix("loop-"))?;

    index.parse().ok()
}

fn read_level_frames(level_dir: &Path) -> Result<Vec<String>, String> {
    let mut indexed_frames = Vec::new();

    for entry in fs::read_dir(level_dir).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;

        if !entry.file_type().map_err(|error| error.to_string())?.is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy().into_owned();

        if let Some(index) = frame_index(&file_name) {
            indexed_frames.push((index, entry.path()));
        }
    }

    indexed_frames.sort_by_key(|(index, _)| *index);

    if indexed_frames.len() < MIN_FRAMES_PER_LEVEL {
        return Err("not enough frames".into());
    }

    for (expected, (actual, _)) in indexed_frames.iter().enumerate() {
        if *actual != expected {
            return Err("discontinuous frames".into());
        }
    }

    Ok(indexed_frames
        .into_iter()
        .map(|(_, path)| local_asset_path(&path))
        .collect())
}

fn read_pack(pack_dir: &Path) -> Result<PetPackDto, String> {
    let folder_name = pack_dir
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "invalid folder name".to_string())?;
    let mut levels = Vec::new();

    for level in LEVELS {
        let frames = read_level_frames(&pack_dir.join(format!("lv{level}")))?;

        levels.push(PetPackLevelDto {
            level,
            duration_ms: (frames.len() as u64 * 140).max(900),
            frames,
        });
    }

    Ok(PetPackDto {
        id: format!("local:{folder_name}"),
        name: folder_name.into(),
        description: "本地宠物包".into(),
        levels,
    })
}

fn scan_pet_packs(app: &AppHandle) -> Result<Vec<PetPackDto>, String> {
    let root = pet_packs_dir(app)?;
    let mut packs = Vec::new();

    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;

        if !entry.file_type().map_err(|error| error.to_string())?.is_dir() {
            continue;
        }

        if let Ok(pack) = read_pack(&entry.path()) {
            packs.push(pack);
        }
    }

    packs.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(packs)
}

#[tauri::command]
pub fn list_pet_packs(app: AppHandle) -> Result<Vec<PetPackDto>, String> {
    scan_pet_packs(&app)
}

#[tauri::command]
pub fn open_pet_packs_folder(app: AppHandle) -> Result<String, String> {
    let path = pet_packs_dir(&app)?;

    #[cfg(target_os = "windows")]
    let result = Command::new("explorer").arg(&path).spawn();

    #[cfg(target_os = "macos")]
    let result = Command::new("open").arg(&path).spawn();

    #[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
    let result = Command::new("xdg-open").arg(&path).spawn();

    result.map_err(|error| error.to_string())?;
    Ok(local_asset_path(&path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_frame(path: &Path) {
        fs::write(path, b"png").unwrap();
    }

    fn create_level(root: &Path, level: u8, prefix: &str, frames: &[usize]) {
        let level_dir = root.join(format!("lv{level}"));
        fs::create_dir_all(&level_dir).unwrap();

        for frame in frames {
            write_frame(&level_dir.join(format!("{prefix}-{frame}.png")));
        }
    }

    #[test]
    fn read_pack_accepts_three_complete_levels() {
        let tempdir = tempdir().unwrap();
        let pack_dir = tempdir.path().join("cat");

        for level in LEVELS {
            create_level(&pack_dir, level, "frame", &[0, 1, 2]);
        }

        let pack = read_pack(&pack_dir).unwrap();

        assert_eq!(pack.id, "local:cat");
        assert_eq!(pack.levels.len(), 3);
        assert_eq!(pack.levels[0].frames.len(), 3);
    }

    #[test]
    fn read_pack_accepts_loop_prefixed_frames() {
        let tempdir = tempdir().unwrap();
        let pack_dir = tempdir.path().join("octo");

        for level in LEVELS {
            create_level(&pack_dir, level, "loop", &[0, 1, 2]);
        }

        let pack = read_pack(&pack_dir).unwrap();

        assert_eq!(pack.id, "local:octo");
        assert!(pack.levels[0].frames[0].ends_with("loop-0.png"));
    }

    #[test]
    fn read_pack_rejects_discontinuous_frames() {
        let tempdir = tempdir().unwrap();
        let pack_dir = tempdir.path().join("cat");

        create_level(&pack_dir, 1, "frame", &[0, 1, 3]);
        create_level(&pack_dir, 2, "frame", &[0, 1]);
        create_level(&pack_dir, 3, "frame", &[0, 1]);

        assert!(read_pack(&pack_dir).is_err());
    }

    #[test]
    fn read_pack_rejects_single_frame_levels() {
        let tempdir = tempdir().unwrap();
        let pack_dir = tempdir.path().join("cat");

        create_level(&pack_dir, 1, "frame", &[0]);
        create_level(&pack_dir, 2, "frame", &[0, 1]);
        create_level(&pack_dir, 3, "frame", &[0, 1]);

        assert!(read_pack(&pack_dir).is_err());
    }
}
