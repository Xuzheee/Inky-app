use std::{path::Path, sync::Mutex};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const DATABASE_VERSION: u8 = 2;
const DEFAULT_PET_NAME: &str = "pet";
const DEFAULT_PET_SET_ID: &str = "builtin:inky";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FocusFlowPersistedState {
    pub version: u8,
    pub tasks: Vec<TaskDto>,
    pub mood: String,
    pub xp: i64,
    pub pet_name: String,
    pub pet_set_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub category: String,
    pub priority: String,
    pub completed: bool,
    pub due: Option<String>,
    pub completed_pomodoros: i64,
}

pub struct PersistenceState {
    connection: Mutex<Connection>,
}

impl PersistenceState {
    pub fn new(connection: Connection) -> Self {
        Self {
            connection: Mutex::new(connection),
        }
    }
}

pub fn default_state() -> FocusFlowPersistedState {
    FocusFlowPersistedState {
        version: DATABASE_VERSION,
        pet_name: DEFAULT_PET_NAME.into(),
        pet_set_id: DEFAULT_PET_SET_ID.into(),
        mood: "好".into(),
        xp: 10,
        tasks: vec![
            TaskDto {
                id: "1".into(),
                title: "回复导师邮件".into(),
                category: "work".into(),
                priority: "high".into(),
                completed: false,
                due: Some("今天 16:00".into()),
                completed_pomodoros: 0,
            },
            TaskDto {
                id: "2".into(),
                title: "整理读书笔记".into(),
                category: "study".into(),
                priority: "medium".into(),
                completed: false,
                due: None,
                completed_pomodoros: 0,
            },
            TaskDto {
                id: "3".into(),
                title: "买明天早饭食材".into(),
                category: "life".into(),
                priority: "low".into(),
                completed: true,
                due: Some("明早".into()),
                completed_pomodoros: 0,
            },
        ],
    }
}

pub fn open_database(path: impl AsRef<Path>) -> Result<Connection, rusqlite::Error> {
    let mut connection = Connection::open(path)?;
    initialize_schema(&mut connection)?;
    Ok(connection)
}

fn initialize_schema(connection: &mut Connection) -> Result<(), rusqlite::Error> {
    connection.pragma_update(None, "foreign_keys", "ON")?;
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if version == 0 {
        connection.execute_batch(
            "
            CREATE TABLE app_state (
              id INTEGER PRIMARY KEY CHECK (id = 1),
              mood TEXT NOT NULL CHECK (mood IN ('好', '一般', '烦')),
              xp INTEGER NOT NULL CHECK (xp >= 0),
              pet_name TEXT NOT NULL,
              pet_set_id TEXT NOT NULL DEFAULT 'builtin:inky'
            );

            CREATE TABLE tasks (
              id TEXT PRIMARY KEY,
              title TEXT NOT NULL,
              category TEXT NOT NULL CHECK (category IN ('work', 'study', 'life', 'idea')),
              priority TEXT NOT NULL CHECK (priority IN ('high', 'medium', 'low')),
              completed INTEGER NOT NULL CHECK (completed IN (0, 1)),
              due TEXT NULL,
              completed_pomodoros INTEGER NOT NULL CHECK (completed_pomodoros >= 0),
              sort_order INTEGER NOT NULL
            );

            PRAGMA user_version = 2;
            ",
        )?;
        let defaults = default_state();
        save_state(connection, &defaults)?;
    } else if version == 1 {
        connection.execute_batch(
            "
            ALTER TABLE app_state ADD COLUMN pet_set_id TEXT NOT NULL DEFAULT 'builtin:inky';
            PRAGMA user_version = 2;
            ",
        )?;
    } else if version > i64::from(DATABASE_VERSION) {
        return Err(rusqlite::Error::InvalidQuery);
    }

    Ok(())
}

pub fn load_state(connection: &Connection) -> Result<FocusFlowPersistedState, rusqlite::Error> {
    let (mood, xp, pet_name, pet_set_id): (String, i64, String, String) = connection.query_row(
        "SELECT mood, xp, pet_name, pet_set_id FROM app_state WHERE id = 1",
        [],
        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
    )?;

    let mut statement = connection.prepare(
        "
        SELECT id, title, category, priority, completed, due, completed_pomodoros
        FROM tasks
        ORDER BY sort_order ASC
        ",
    )?;
    let tasks = statement
        .query_map([], |row| {
            Ok(TaskDto {
                id: row.get(0)?,
                title: row.get(1)?,
                category: row.get(2)?,
                priority: row.get(3)?,
                completed: row.get::<_, i64>(4)? == 1,
                due: row.get(5)?,
                completed_pomodoros: row.get(6)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(FocusFlowPersistedState {
        version: DATABASE_VERSION,
        tasks,
        mood,
        xp,
        pet_name,
        pet_set_id,
    })
}

pub fn save_state(connection: &mut Connection, state: &FocusFlowPersistedState) -> Result<(), rusqlite::Error> {
    validate_state(state)?;
    let transaction = connection.transaction()?;
    let pet_name = normalize_pet_name(&state.pet_name);
    let pet_set_id = normalize_pet_set_id(&state.pet_set_id);

    transaction.execute(
        "
        INSERT INTO app_state (id, mood, xp, pet_name, pet_set_id)
        VALUES (1, ?1, ?2, ?3, ?4)
        ON CONFLICT(id) DO UPDATE SET mood = excluded.mood, xp = excluded.xp, pet_name = excluded.pet_name, pet_set_id = excluded.pet_set_id
        ",
        params![state.mood, state.xp, pet_name, pet_set_id],
    )?;
    transaction.execute("DELETE FROM tasks", [])?;

    {
        let mut statement = transaction.prepare(
            "
            INSERT INTO tasks (id, title, category, priority, completed, due, completed_pomodoros, sort_order)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
            ",
        )?;

        for (index, task) in state.tasks.iter().enumerate() {
            statement.execute(params![
                task.id,
                task.title,
                task.category,
                task.priority,
                if task.completed { 1 } else { 0 },
                task.due,
                task.completed_pomodoros,
                index as i64,
            ])?;
        }
    }

    transaction.commit()
}

fn validate_state(state: &FocusFlowPersistedState) -> Result<(), rusqlite::Error> {
    if state.version != DATABASE_VERSION || !is_mood(&state.mood) || state.xp < 0 {
        return Err(rusqlite::Error::InvalidQuery);
    }

    for task in &state.tasks {
        if task.id.trim().is_empty()
            || task.title.trim().is_empty()
            || !is_category(&task.category)
            || !is_priority(&task.priority)
            || task.completed_pomodoros < 0
        {
            return Err(rusqlite::Error::InvalidQuery);
        }
    }

    Ok(())
}

fn normalize_pet_name(pet_name: &str) -> String {
    let trimmed = pet_name.trim();

    if trimmed.is_empty() {
        DEFAULT_PET_NAME.into()
    } else {
        trimmed.into()
    }
}

fn normalize_pet_set_id(pet_set_id: &str) -> String {
    if is_pet_set_id(pet_set_id) {
        pet_set_id.into()
    } else {
        DEFAULT_PET_SET_ID.into()
    }
}

fn is_pet_set_id(value: &str) -> bool {
    value == DEFAULT_PET_SET_ID
        || value
            .strip_prefix("local:")
            .is_some_and(|name| !name.trim().is_empty() && !name.contains(['/', '\\']))
}

fn is_mood(value: &str) -> bool {
    matches!(value, "好" | "一般" | "烦")
}

fn is_category(value: &str) -> bool {
    matches!(value, "work" | "study" | "life" | "idea")
}

fn is_priority(value: &str) -> bool {
    matches!(value, "high" | "medium" | "low")
}

#[tauri::command]
pub fn load_focus_flow_state(state: tauri::State<'_, PersistenceState>) -> Result<FocusFlowPersistedState, String> {
    let connection = state
        .connection
        .lock()
        .map_err(|error| format!("failed to lock database: {error}"))?;

    load_state(&connection).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn save_focus_flow_state(
    state: tauri::State<'_, PersistenceState>,
    payload: FocusFlowPersistedState,
) -> Result<(), String> {
    let mut connection = state
        .connection
        .lock()
        .map_err(|error| format!("failed to lock database: {error}"))?;

    save_state(&mut connection, &payload).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::{tempdir, TempDir};

    struct TestDatabase {
        _tempdir: TempDir,
        connection: Connection,
    }

    fn test_database() -> TestDatabase {
        let tempdir = tempdir().unwrap();
        let connection = open_database(tempdir.path().join("focusflow.sqlite3")).unwrap();

        TestDatabase {
            _tempdir: tempdir,
            connection,
        }
    }

    fn custom_state() -> FocusFlowPersistedState {
        FocusFlowPersistedState {
            version: 2,
            mood: "一般".into(),
            xp: 42,
            pet_name: "小蓝".into(),
            pet_set_id: "local:cat".into(),
            tasks: vec![
                TaskDto {
                    id: "later".into(),
                    title: "整理读书笔记".into(),
                    category: "study".into(),
                    priority: "medium".into(),
                    completed: false,
                    due: None,
                    completed_pomodoros: 2,
                },
                TaskDto {
                    id: "first".into(),
                    title: "回复导师邮件".into(),
                    category: "work".into(),
                    priority: "high".into(),
                    completed: true,
                    due: Some("今天 16:00".into()),
                    completed_pomodoros: 1,
                },
            ],
        }
    }

    #[test]
    fn first_run_initializes_schema_and_default_state() {
        let database = test_database();
        let state = load_state(&database.connection).unwrap();
        let schema_version: i64 = database.connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();

        assert_eq!(schema_version, 2);
        assert_eq!(state.version, 2);
        assert_eq!(state.mood, "好");
        assert_eq!(state.xp, 10);
        assert_eq!(state.pet_name, "pet");
        assert_eq!(state.pet_set_id, "builtin:inky");
        assert_eq!(state.tasks.len(), 3);
        assert_eq!(state.tasks[0].title, "回复导师邮件");
    }

    #[test]
    fn saving_then_loading_preserves_state_and_task_order() {
        let mut database = test_database();
        let expected = custom_state();

        save_state(&mut database.connection, &expected).unwrap();
        let loaded = load_state(&database.connection).unwrap();

        assert_eq!(loaded, expected);
    }

    #[test]
    fn save_rejects_invalid_enums_and_negative_numbers() {
        let mut database = test_database();
        let mut invalid = custom_state();
        invalid.mood = "unknown".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.tasks[0].category = "other".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.xp = -1;
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.tasks[0].completed_pomodoros = -1;
        assert!(save_state(&mut database.connection, &invalid).is_err());
    }

    #[test]
    fn blank_pet_name_normalizes_to_default_pet_name() {
        let mut database = test_database();
        let mut state = custom_state();
        state.pet_name = "   ".into();

        save_state(&mut database.connection, &state).unwrap();
        let loaded = load_state(&database.connection).unwrap();

        assert_eq!(loaded.pet_name, "pet");
    }

    #[test]
    fn unknown_pet_set_normalizes_to_default_pet_set() {
        let mut database = test_database();
        let mut state = custom_state();
        state.pet_set_id = "unknown".into();

        save_state(&mut database.connection, &state).unwrap();
        let loaded = load_state(&database.connection).unwrap();

        assert_eq!(loaded.pet_set_id, "builtin:inky");
    }

    #[test]
    fn version_one_database_migrates_to_pet_set_schema() {
        let tempdir = tempdir().unwrap();
        let database_path = tempdir.path().join("focusflow.sqlite3");
        let connection = Connection::open(&database_path).unwrap();
        connection
            .execute_batch(
                "
                CREATE TABLE app_state (
                  id INTEGER PRIMARY KEY CHECK (id = 1),
                  mood TEXT NOT NULL CHECK (mood IN ('好', '一般', '烦')),
                  xp INTEGER NOT NULL CHECK (xp >= 0),
                  pet_name TEXT NOT NULL
                );

                CREATE TABLE tasks (
                  id TEXT PRIMARY KEY,
                  title TEXT NOT NULL,
                  category TEXT NOT NULL CHECK (category IN ('work', 'study', 'life', 'idea')),
                  priority TEXT NOT NULL CHECK (priority IN ('high', 'medium', 'low')),
                  completed INTEGER NOT NULL CHECK (completed IN (0, 1)),
                  due TEXT NULL,
                  completed_pomodoros INTEGER NOT NULL CHECK (completed_pomodoros >= 0),
                  sort_order INTEGER NOT NULL
                );

                INSERT INTO app_state (id, mood, xp, pet_name) VALUES (1, '一般', 42, 'Inky');
                PRAGMA user_version = 1;
                ",
            )
            .unwrap();
        drop(connection);

        let connection = open_database(database_path).unwrap();
        let schema_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let state = load_state(&connection).unwrap();

        assert_eq!(schema_version, 2);
        assert_eq!(state.pet_set_id, "builtin:inky");
        assert_eq!(state.mood, "一般");
        assert_eq!(state.xp, 42);
        assert_eq!(state.pet_name, "Inky");
    }
}
