use std::{path::Path, sync::Mutex};

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

const DATABASE_VERSION: u8 = 4;
const DEFAULT_PET_NAME: &str = "Inky";
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
    pub inbox_items: Vec<InboxItemDto>,
    pub show_focus_return: bool,
    pub has_completed_pet_naming: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InboxItemDto {
    pub id: String,
    pub text: String,
    pub created_at: String,
    pub status: String,
    pub converted_task_id: Option<String>,
    pub date: String,
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
        inbox_items: Vec::new(),
        show_focus_return: true,
        has_completed_pet_naming: false,
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
              pet_set_id TEXT NOT NULL DEFAULT 'builtin:inky',
              show_focus_return INTEGER NOT NULL DEFAULT 1 CHECK (show_focus_return IN (0, 1)),
              has_completed_pet_naming INTEGER NOT NULL DEFAULT 0 CHECK (has_completed_pet_naming IN (0, 1))
            );

            CREATE TABLE inbox_items (
              id TEXT PRIMARY KEY,
              text TEXT NOT NULL,
              created_at TEXT NOT NULL,
              status TEXT NOT NULL CHECK (status IN ('pending', 'converted', 'archived', 'deleted')),
              converted_task_id TEXT NULL,
              date TEXT NOT NULL,
              sort_order INTEGER NOT NULL
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

            PRAGMA user_version = 4;
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
        migrate_v2_to_v3(connection)?;
        migrate_v3_to_v4(connection)?;
    } else if version == 2 {
        migrate_v2_to_v3(connection)?;
        migrate_v3_to_v4(connection)?;
    } else if version == 3 {
        migrate_v3_to_v4(connection)?;
    } else if version > i64::from(DATABASE_VERSION) {
        return Err(rusqlite::Error::InvalidQuery);
    }

    Ok(())
}

fn migrate_v2_to_v3(connection: &Connection) -> Result<(), rusqlite::Error> {
    if !app_state_has_column(connection, "show_focus_return")? {
        connection.execute_batch(
            "
            ALTER TABLE app_state ADD COLUMN show_focus_return INTEGER NOT NULL DEFAULT 1 CHECK (show_focus_return IN (0, 1));
            ",
        )?;
    }

    connection.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS inbox_items (
          id TEXT PRIMARY KEY,
          text TEXT NOT NULL,
          created_at TEXT NOT NULL,
          status TEXT NOT NULL CHECK (status IN ('pending', 'converted', 'archived', 'deleted')),
          converted_task_id TEXT NULL,
          date TEXT NOT NULL,
          sort_order INTEGER NOT NULL
        );

        PRAGMA user_version = 3;
        ",
    )
}

fn migrate_v3_to_v4(connection: &Connection) -> Result<(), rusqlite::Error> {
    if !app_state_has_column(connection, "has_completed_pet_naming")? {
        connection.execute_batch(
            "
            ALTER TABLE app_state ADD COLUMN has_completed_pet_naming INTEGER NOT NULL DEFAULT 0 CHECK (has_completed_pet_naming IN (0, 1));
            ",
        )?;
    }

    connection.execute_batch("PRAGMA user_version = 4;")
}

fn app_state_has_column(connection: &Connection, column_name: &str) -> Result<bool, rusqlite::Error> {
    let mut statement = connection.prepare("PRAGMA table_info(app_state)")?;
    let mut rows = statement.query([])?;

    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;

        if name == column_name {
            return Ok(true);
        }
    }

    Ok(false)
}

pub fn load_state(connection: &Connection) -> Result<FocusFlowPersistedState, rusqlite::Error> {
    let (mood, xp, pet_name, pet_set_id, show_focus_return, has_completed_pet_naming):
        (String, i64, String, String, bool, bool) = connection.query_row(
        "SELECT mood, xp, pet_name, pet_set_id, show_focus_return, has_completed_pet_naming FROM app_state WHERE id = 1",
        [],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get::<_, i64>(4)? == 1,
                row.get::<_, i64>(5)? == 1,
            ))
        },
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

    let mut statement = connection.prepare(
        "
        SELECT id, text, created_at, status, converted_task_id, date
        FROM inbox_items
        ORDER BY sort_order ASC
        ",
    )?;
    let inbox_items = statement
        .query_map([], |row| {
            Ok(InboxItemDto {
                id: row.get(0)?,
                text: row.get(1)?,
                created_at: row.get(2)?,
                status: row.get(3)?,
                converted_task_id: row.get(4)?,
                date: row.get(5)?,
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
        inbox_items,
        show_focus_return,
        has_completed_pet_naming,
    })
}

pub fn save_state(
    connection: &mut Connection,
    state: &FocusFlowPersistedState,
) -> Result<(), rusqlite::Error> {
    validate_state(state)?;
    let transaction = connection.transaction()?;
    let pet_name = normalize_pet_name(&state.pet_name);
    let pet_set_id = normalize_pet_set_id(&state.pet_set_id);

    transaction.execute(
        "
        INSERT INTO app_state (id, mood, xp, pet_name, pet_set_id, show_focus_return, has_completed_pet_naming)
        VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(id) DO UPDATE SET mood = excluded.mood, xp = excluded.xp, pet_name = excluded.pet_name, pet_set_id = excluded.pet_set_id, show_focus_return = excluded.show_focus_return, has_completed_pet_naming = excluded.has_completed_pet_naming
        ",
        params![
            state.mood,
            state.xp,
            pet_name,
            pet_set_id,
            if state.show_focus_return { 1 } else { 0 },
            if state.has_completed_pet_naming { 1 } else { 0 },
        ],
    )?;
    transaction.execute("DELETE FROM tasks", [])?;
    transaction.execute("DELETE FROM inbox_items", [])?;

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

    {
        let mut statement = transaction.prepare(
            "
            INSERT INTO inbox_items (id, text, created_at, status, converted_task_id, date, sort_order)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ",
        )?;

        for (index, item) in state.inbox_items.iter().enumerate() {
            statement.execute(params![
                item.id,
                item.text,
                item.created_at,
                item.status,
                item.converted_task_id,
                item.date,
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

    for item in &state.inbox_items {
        if item.id.trim().is_empty()
            || item.text.trim().is_empty()
            || item.created_at.trim().is_empty()
            || item.date.trim().is_empty()
            || !is_inbox_status(&item.status)
            || (item.status == "converted") != item.converted_task_id.is_some()
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

fn is_inbox_status(value: &str) -> bool {
    matches!(value, "pending" | "converted" | "archived" | "deleted")
}

#[tauri::command]
pub fn load_focus_flow_state(
    state: tauri::State<'_, PersistenceState>,
) -> Result<FocusFlowPersistedState, String> {
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
            version: 4,
            mood: "一般".into(),
            xp: 42,
            pet_name: "小蓝".into(),
            pet_set_id: "local:cat".into(),
            inbox_items: vec![InboxItemDto {
                id: "inbox-1".into(),
                text: "Capture this later".into(),
                created_at: "2026-05-16T10:00:00Z".into(),
                status: "pending".into(),
                converted_task_id: None,
                date: "2026-05-16".into(),
            }],
            show_focus_return: true,
            has_completed_pet_naming: true,
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
        let schema_version: i64 = database
            .connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();

        let (show_focus_return, has_completed_pet_naming): (i64, i64) = database
            .connection
            .query_row(
                "SELECT show_focus_return, has_completed_pet_naming FROM app_state WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        let inbox_count: i64 = database
            .connection
            .query_row("SELECT COUNT(*) FROM inbox_items", [], |row| row.get(0))
            .unwrap();

        assert_eq!(schema_version, 4);
        assert_eq!(state.version, 4);
        assert_eq!(show_focus_return, 1);
        assert_eq!(has_completed_pet_naming, 0);
        assert!(!state.has_completed_pet_naming);
        assert_eq!(inbox_count, 0);
        assert_eq!(state.mood, "好");
        assert_eq!(state.xp, 10);
        assert_eq!(state.pet_name, "Inky");
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
    fn save_preserves_inbox_item_order() {
        let mut database = test_database();
        let mut expected = custom_state();
        expected.inbox_items.push(InboxItemDto {
            id: "inbox-2".into(),
            text: "Second capture".into(),
            created_at: "2026-05-16T11:00:00Z".into(),
            status: "archived".into(),
            converted_task_id: None,
            date: "2026-05-16".into(),
        });
        expected.inbox_items.push(InboxItemDto {
            id: "inbox-3".into(),
            text: "Converted capture".into(),
            created_at: "2026-05-16T12:00:00Z".into(),
            status: "converted".into(),
            converted_task_id: Some("first".into()),
            date: "2026-05-16".into(),
        });
        expected.show_focus_return = false;

        save_state(&mut database.connection, &expected).unwrap();
        let loaded = load_state(&database.connection).unwrap();

        assert_eq!(loaded.inbox_items, expected.inbox_items);
        assert!(!loaded.show_focus_return);
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

        let mut invalid = custom_state();
        invalid.inbox_items[0].status = "unknown".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.inbox_items[0].text = "   ".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.inbox_items[0].id = "   ".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.inbox_items[0].created_at = "   ".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.inbox_items[0].date = "   ".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.inbox_items[0].status = "converted".into();
        assert!(save_state(&mut database.connection, &invalid).is_err());

        let mut invalid = custom_state();
        invalid.inbox_items[0].converted_task_id = Some("task-1".into());
        assert!(save_state(&mut database.connection, &invalid).is_err());
    }

    #[test]
    fn blank_pet_name_normalizes_to_default_pet_name() {
        let mut database = test_database();
        let mut state = custom_state();
        state.pet_name = "   ".into();

        save_state(&mut database.connection, &state).unwrap();
        let loaded = load_state(&database.connection).unwrap();

        assert_eq!(loaded.pet_name, "Inky");
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

        assert_eq!(schema_version, 4);
        assert_eq!(state.pet_set_id, "builtin:inky");
        assert_eq!(state.show_focus_return, true);
        assert!(!state.has_completed_pet_naming);
        assert!(state.inbox_items.is_empty());
        assert_eq!(state.mood, "一般");
        assert_eq!(state.xp, 42);
        assert_eq!(state.pet_name, "Inky");
    }

    #[test]
    fn version_two_database_migrates_to_inbox_schema() {
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

                INSERT INTO app_state (id, mood, xp, pet_name, pet_set_id) VALUES (1, '烦', 7, 'Inky', 'local:fox');
                PRAGMA user_version = 2;
                ",
            )
            .unwrap();
        drop(connection);

        let connection = open_database(database_path).unwrap();
        let schema_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let show_focus_return: i64 = connection
            .query_row(
                "SELECT show_focus_return FROM app_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let inbox_count: i64 = connection
            .query_row("SELECT COUNT(*) FROM inbox_items", [], |row| row.get(0))
            .unwrap();
        let state = load_state(&connection).unwrap();

        assert_eq!(schema_version, 4);
        assert_eq!(show_focus_return, 1);
        assert_eq!(inbox_count, 0);
        assert!(state.show_focus_return);
        assert!(!state.has_completed_pet_naming);
        assert!(state.inbox_items.is_empty());
        assert_eq!(state.pet_set_id, "local:fox");
        assert_eq!(state.mood, "烦");
    }

    #[test]
    fn version_three_database_migrates_to_pet_naming_schema() {
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
                  pet_name TEXT NOT NULL,
                  pet_set_id TEXT NOT NULL DEFAULT 'builtin:inky',
                  show_focus_return INTEGER NOT NULL DEFAULT 1 CHECK (show_focus_return IN (0, 1))
                );

                CREATE TABLE inbox_items (
                  id TEXT PRIMARY KEY,
                  text TEXT NOT NULL,
                  created_at TEXT NOT NULL,
                  status TEXT NOT NULL CHECK (status IN ('pending', 'converted', 'archived', 'deleted')),
                  converted_task_id TEXT NULL,
                  date TEXT NOT NULL,
                  sort_order INTEGER NOT NULL
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

                INSERT INTO app_state (id, mood, xp, pet_name, pet_set_id, show_focus_return)
                VALUES (1, '好', 12, '小蓝', 'builtin:inky', 0);
                INSERT INTO inbox_items (id, text, created_at, status, converted_task_id, date, sort_order)
                VALUES ('inbox-1', 'Remember this', '2026-05-20T10:00:00Z', 'pending', NULL, '2026-05-20', 0);
                PRAGMA user_version = 3;
                ",
            )
            .unwrap();
        drop(connection);

        let connection = open_database(database_path).unwrap();
        let schema_version: i64 = connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        let has_completed_pet_naming: i64 = connection
            .query_row(
                "SELECT has_completed_pet_naming FROM app_state WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let state = load_state(&connection).unwrap();

        assert_eq!(schema_version, 4);
        assert_eq!(has_completed_pet_naming, 0);
        assert!(!state.has_completed_pet_naming);
        assert!(!state.show_focus_return);
        assert_eq!(state.pet_name, "小蓝");
        assert_eq!(state.inbox_items.len(), 1);
    }
}
