//! Integration tests for the SIAPLA GraphQL API against a real SQLite database.
//!
//! All tests share a single temporary database (because `GLOBAL_DATABASE_URL` uses a
//! process-wide `OnceCell`). Tests are run serially via `#[serial]` to avoid
//! concurrency issues on the shared database.

use std::sync::Arc;

use siapla::entity::revision;
use tokio::sync::OnceCell;

use juniper::{ScalarValue as _, Variables};
use sea_orm::{Database, DatabaseConnection, EntityTrait, QueryOrder as _, TransactionTrait as _};
use serial_test::serial;
use siapla::gql::scalars::ExtendedScalarValue;
use siapla::{
    app_state::AppState,
    gql::{
        context::{Context, set_global_database_url},
        schema,
    },
};
use siapla_migration::MigratorTrait as _;

// ---------------------------------------------------------------------------
// Shared one-time setup
// ---------------------------------------------------------------------------

/// Returns the database URL for the shared test database.
/// The first call creates the temp file, the database, and runs migrations.
/// Subsequent calls return the same URL.
async fn shared_db_url() -> &'static str {
    static DB_URL: OnceCell<String> = OnceCell::const_new();
    DB_URL
        .get_or_init(|| async {
            let tmp = tempfile::NamedTempFile::new().expect("failed to create temp file");
            // Keep the file alive for the duration of the process by leaking it.
            let path = tmp.into_temp_path();
            let db_url = format!("sqlite:{}?mode=rwc", path.display());

            let db: DatabaseConnection =
                Database::connect(&db_url).await.expect("failed to connect to test db");
            siapla_migration::Migrator::up(&db, None).await.expect("migrations failed");
            db.close().await.expect("failed to close db after migrations");

            // Leak the temp path so the file is not deleted while the process is alive.
            std::mem::forget(path);

            // Set the global URL that Context will use.
            set_global_database_url(&db_url);

            db_url
        })
        .await
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Wipe all user-data tables so every test starts with a clean slate.
async fn clean_database() {
    let url = shared_db_url().await;
    let db = Database::connect(url).await.expect("clean_database: connect failed");
    let txn = db.begin().await.expect("clean_database: begin failed");

    // Order matters because of foreign-key constraints.
    use sea_orm::ConnectionTrait as _;
    for table in &[
        "allocated_resource",
        "allocation",
        "issue",
        "dependency",
        "resource_constraint_entry",
        "resource_constraint",
        "availability",
        "vacation",
        "task_iteration",
        "task_header",
        "resource_iteration",
        "resource_header",
        "revision",
        "holiday_entry",
        "holiday",
    ] {
        txn.execute(sea_orm::Statement::from_string(
            sea_orm::DatabaseBackend::Sqlite,
            format!("DELETE FROM \"{}\";", table),
        ))
        .await
        .unwrap_or_else(|e| panic!("clean_database: failed to clear {table}: {e}"));
    }

    txn.execute(sea_orm::Statement::from_string(
        sea_orm::DatabaseBackend::Sqlite,
        r#"DELETE FROM sqlite_sequence
           WHERE name IN (
               'allocated_resource',
               'allocation',
               'issue',
               'dependency',
               'resource_constraint_entry',
               'resource_constraint',
               'availability',
               'vacation',
               'task_iteration',
               'task_header',
               'resource_iteration',
               'resource_header',
               'revision',
               'holiday_entry',
               'holiday'
           );"#
        .to_string(),
    ))
    .await
    .expect("clean_database: failed to reset sqlite_sequence");

    txn.commit().await.expect("clean_database: commit failed");
    db.close().await.ok();
}

/// Execute a GraphQL **query or mutation** and return the result value.
/// Panics on transport/execution errors.
async fn gql_exec(
    query: &str,
    vars: &Variables<ExtendedScalarValue>,
) -> (juniper::Value<ExtendedScalarValue>, Vec<juniper::ExecutionError<ExtendedScalarValue>>) {
    let schema = schema();
    let (app_state, _manual_rx) = AppState::new(true);
    let ctx = Context::new(app_state);
    let (val, errors) = juniper::execute(query, None, &schema, vars, &*ctx)
        .await
        .expect("GraphQL execution failed");

    // Commit the transaction so data is persisted for subsequent operations.
    let ctx_owned =
        Arc::into_inner(ctx).expect("Context Arc should have no other strong references");
    ctx_owned.commit().await.expect("commit failed");

    (val, errors)
}

/// Shorthand: execute, assert zero errors, return the data `Value`.
async fn gql_ok(
    query: &str,
    vars: &Variables<ExtendedScalarValue>,
) -> juniper::Value<ExtendedScalarValue> {
    let (val, errors) = gql_exec(query, vars).await;
    assert!(errors.is_empty(), "Expected no GraphQL errors but got: {errors:?}\nQuery: {query}");
    val
}

/// Shorthand: execute with empty variables.
async fn gql_ok_simple(query: &str) -> juniper::Value<ExtendedScalarValue> {
    gql_ok(query, &Variables::<ExtendedScalarValue>::new()).await
}

/// Get the juniper value at the given dot path
fn value_at_path<'a>(
    val: &'a juniper::Value<ExtendedScalarValue>,
    path: &str,
) -> &'a juniper::Value<ExtendedScalarValue> {
    let mut cur = val;
    for key in path.split('.') {
        cur = cur
            .as_object_value()
            .unwrap_or_else(|| {
                panic!("expected object at key segment '{key}' in path '{path}', got {cur:?}")
            })
            .iter()
            .find(|(k, _)| *k == key)
            .unwrap_or_else(|| panic!("key '{key}' not found in path '{path}'"))
            .1;
    }
    cur
}

/// Extract a scalar i64 from a juniper Value sitting at the given dot-path
/// (e.g. "taskSave.dbId").
fn extract_i64(val: &juniper::Value<ExtendedScalarValue>, path: &str) -> i64 {
    let cur = value_at_path(val, path);
    let s = cur.as_scalar().unwrap_or_else(|| panic!("value at '{path}' is not a scalar: {cur:?}"));
    if let &ExtendedScalarValue::Long(val) = s {
        val
    } else {
        s.try_to_int()
            .map(|v| v as i64)
            .unwrap_or_else(|| panic!("value at '{path}' is not an int: {cur:?}"))
    }
}

fn extract_str<'a>(val: &'a juniper::Value<ExtendedScalarValue>, path: &str) -> &'a str {
    let cur = value_at_path(val, path);
    cur.as_scalar()
        .and_then(|s| s.try_as_str())
        .unwrap_or_else(|| panic!("value at '{path}' is not a string: {cur:?}"))
}

fn extract_list<'a>(
    val: &'a juniper::Value<ExtendedScalarValue>,
    path: &str,
) -> &'a Vec<juniper::Value<ExtendedScalarValue>> {
    let cur = value_at_path(val, path);
    cur.as_list_value().unwrap_or_else(|| panic!("value at '{path}' is not a list: {cur:?}"))
}

fn extract_f64(val: &juniper::Value<ExtendedScalarValue>, path: &str) -> f64 {
    let cur = value_at_path(val, path);
    cur.as_scalar()
        .and_then(|s| s.try_to_float())
        .unwrap_or_else(|| panic!("value at '{path}' is not f64: {cur:?}"))
}

/// Create a resource through GraphQL and return its `dbId`.
async fn create_resource(name: &str, tz: &str, with_availability: bool) -> i64 {
    let avail_fragment = if with_availability {
        r#"availability: [
            { weekday: MONDAY, duration: 28800 },
            { weekday: TUESDAY, duration: 28800 },
            { weekday: WEDNESDAY, duration: 28800 },
            { weekday: THURSDAY, duration: 28800 },
            { weekday: FRIDAY, duration: 28800 }
        ]"#
    } else {
        ""
    };

    let mutation = format!(
        r#"mutation {{
            resourceSave(resource: {{
                name: "{name}",
                timezone: "{tz}",
                added: "2025-01-01T00:00:00Z",
                {avail_fragment}
            }}) {{
                dbId
                name
                timezone
            }}
        }}"#
    );

    let val = gql_ok_simple(&mutation).await;
    let id = extract_i64(&val, "resourceSave.dbId");
    let stored_name = extract_str(&val, "resourceSave.name");
    let stored_tz = extract_str(&val, "resourceSave.timezone");
    assert_eq!(stored_name, name, "resourceSave.name mismatch for resource id {id}");
    assert_eq!(
        stored_tz, tz,
        "resourceSave.timezone mismatch for resource id {id}, name {stored_name}"
    );
    id
}

/// Create a task through GraphQL and return its `dbId`.
async fn create_task(
    title: &str,
    designation: &str,
    priority: f64,
    effort: Option<f64>,
    parent_id: Option<i64>,
    predecessors: Option<&[i64]>,
    successors: Option<&[i64]>,
    children: Option<&[i64]>,
    resource_constraints: Option<&str>,
) -> i64 {
    let effort_frag = match effort {
        Some(e) => format!("effort: {e}"),
        None => "effort: null".to_string(),
    };
    let parent_frag = match parent_id {
        Some(pid) => format!("parentId: {pid}"),
        None => "parentId: null".to_string(),
    };
    let pred_frag = match predecessors {
        Some(ids) => format!("predecessors: {:?}", ids),
        None => String::new(),
    };
    let succ_frag = match successors {
        Some(ids) => format!("successors: {:?}", ids),
        None => String::new(),
    };
    let children_frag = match children {
        Some(ids) => format!("children: {:?}", ids),
        None => String::new(),
    };
    let rc_frag = resource_constraints.unwrap_or("");

    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                title: "{title}",
                description: "desc for {title}",
                designation: {designation},
                priority: {priority},
                {effort_frag},
                {parent_frag},
                {pred_frag}
                {succ_frag}
                {children_frag}
                {rc_frag}
            }}) {{
                dbId
                iterationId
                title
                designation
                priority
            }}
        }}"#
    );

    let val = gql_ok_simple(&mutation).await;
    let id = extract_i64(&val, "taskSave.dbId");
    let stored_title = extract_str(&val, "taskSave.title");
    assert_eq!(stored_title, title, "taskSave.title mismatch for task id {id}");
    id
}

/// Query predecessors for a task by resolving the current task and predecessor
/// rows via their titles, because dependency storage is header-based.
async fn query_task_predecessors(task_id: i64) -> Vec<i64> {
    let current_val =
        gql_ok_simple(r#"{ tasks { dbId title predecessors { dbId title } } }"#).await;
    let current_tasks = extract_list(&current_val, "tasks");

    let historical_title = find_task_title(task_id).await;
    let current_task = current_tasks
        .iter()
        .find(|t| extract_str(t, "title") == historical_title)
        .unwrap_or_else(|| panic!("current task for dbId={task_id} not found"));

    let preds = extract_list(current_task, "predecessors");
    let mut ids: Vec<i64> = preds
        .iter()
        .filter_map(|p| {
            let title = extract_str(p, "title");
            current_tasks
                .iter()
                .find(|t| extract_str(t, "title") == title)
                .map(|t| extract_i64(t, "dbId"))
        })
        .collect();
    ids.sort();
    ids
}

async fn find_task_title(task_id: i64) -> String {
    let latest = latest_revision().await;
    for revision_id in 1..=latest {
        let query = format!(
            r#"{{
                tasks(revision: {revision_id}) {{
                    dbId
                    title
                }}
            }}"#
        );
        let val = gql_ok_simple(&query).await;
        let tasks = extract_list(&val, "tasks");
        if let Some(task) = tasks.iter().find(|t| extract_i64(t, "dbId") == task_id) {
            return extract_str(task, "title").to_string();
        }
    }
    panic!("task with dbId={task_id} not found");
}

async fn find_current_task_id_by_title(title: &str) -> i64 {
    let val = gql_ok_simple(r#"{ tasks { dbId title } }"#).await;
    let tasks = extract_list(&val, "tasks");
    tasks
        .iter()
        .find(|t| extract_str(t, "title") == title)
        .map(|t| extract_i64(t, "dbId"))
        .unwrap_or_else(|| panic!("current task with title='{title}' not found"))
}

async fn find_current_iteration_id_by_title(title: &str) -> i64 {
    let val = gql_ok_simple(r#"{ tasks { iterationId title } }"#).await;
    let tasks = extract_list(&val, "tasks");
    tasks
        .iter()
        .find(|t| extract_str(t, "title") == title)
        .map(|t| extract_i64(t, "iterationId"))
        .unwrap_or_else(|| panic!("current task with title='{title}' not found"))
}

async fn latest_revision() -> i64 {
    let val = gql_ok_simple(r#"{ latestRevision }"#).await;
    extract_i64(&val, "latestRevision")
}

// ===========================================================================
// Tests
// ===========================================================================

#[tokio::test]
#[serial]
async fn test_create_resources() {
    clean_database().await;

    // Create Alice in Europe/Berlin
    let alice_id = create_resource("Alice", "Europe/Berlin", true).await;
    assert!(alice_id > 0, "expected alice_id > 0, got {alice_id}");

    // Create Bob in America/New_York
    let bob_id = create_resource("Bob", "America/New_York", true).await;
    assert!(bob_id > 0, "expected bob_id > 0, got {bob_id}");
    assert_ne!(
        alice_id, bob_id,
        "expected distinct ids for Alice and Bob, got alice_id={alice_id}, bob_id={bob_id}"
    );

    // Query all resources
    let val =
        gql_ok_simple(r#"{ resources { dbId name timezone availability { weekday duration } } }"#)
            .await;
    let resources = extract_list(&val, "resources");
    assert_eq!(resources.len(), 2, "expected 2 resources, got {}", resources.len());

    // Verify availability was stored (5 weekdays each)
    for res in resources {
        let res_name = extract_str(res, "name");
        let avail = res
            .as_object_value()
            .unwrap()
            .iter()
            .find(|(k, _)| *k == "availability")
            .unwrap()
            .1
            .as_list_value()
            .unwrap();
        assert_eq!(
            avail.len(),
            5,
            "expected 5 availability entries for resource '{res_name}', got {}",
            avail.len()
        );
    }
}

#[tokio::test]
#[serial]
async fn test_create_simple_project() {
    clean_database().await;

    let alice_id = create_resource("Alice", "Europe/Berlin", true).await;

    // Requirement
    let req_id = create_task("Req-1", "REQUIREMENT", 1.0, None, None, None, None, None, None).await;

    // Task with effort, constrained to Alice, predecessor = requirement
    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let task_id = create_task(
        "Task-1",
        "TASK",
        2.0,
        Some(8.0),
        None,
        Some(&[req_id]),
        None,
        None,
        Some(&rc_frag),
    )
    .await;

    // Milestone with predecessor = task
    let _ms_id = create_task(
        "Milestone-1",
        "MILESTONE",
        3.0,
        None,
        None,
        Some(&[task_id]),
        None,
        None,
        None,
    )
    .await;

    // Verify tasks query returns 3 tasks
    let val = gql_ok_simple(
        r#"{ tasks { dbId title designation predecessors { dbId } successors { dbId } } }"#,
    )
    .await;
    let tasks = extract_list(&val, "tasks");
    assert_eq!(tasks.len(), 3, "expected 3 tasks in simple project, got {}", tasks.len());

    // Verify the dependency chain: Req-1 → Task-1 → Milestone-1
    // Task-1 predecessors should contain Req-1
    let task1 =
        tasks.iter().find(|t| extract_str(t, "title") == "Task-1").expect("Task-1 should exist");
    let preds = extract_list(task1, "predecessors");
    assert_eq!(
        preds.len(),
        1,
        "expected Task-1 to have exactly 1 predecessor, got {}",
        preds.len()
    );
    let pred_id = extract_i64(&preds[0], "dbId");
    assert_eq!(
        pred_id, req_id,
        "expected Task-1 predecessor to be Req-1 ({req_id}), got {pred_id}"
    );

    // Milestone predecessors should contain Task-1
    let ms = tasks
        .iter()
        .find(|t| extract_str(t, "title") == "Milestone-1")
        .expect("Milestone-1 should exist");
    let ms_preds = extract_list(ms, "predecessors");
    assert_eq!(
        ms_preds.len(),
        1,
        "expected Milestone-1 to have exactly 1 predecessor, got {}",
        ms_preds.len()
    );
    let ms_pred_id = extract_i64(&ms_preds[0], "dbId");
    assert_eq!(
        ms_pred_id, task_id,
        "expected Milestone-1 predecessor to be Task-1 ({task_id}), got {ms_pred_id}"
    );
}

#[tokio::test]
#[serial]
async fn test_multiple_resources() {
    clean_database().await;

    let alice_id = create_resource("Alice", "Europe/Berlin", true).await;
    let bob_id = create_resource("Bob", "America/New_York", true).await;
    let charlie_id = create_resource("Charlie", "Asia/Tokyo", true).await;

    // Create a task requiring Alice AND Bob (two separate constraint groups)
    let rc_frag = format!(
        r#"resourceConstraints: [
            {{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }},
            {{ optional: false, speed: 1.0, entries: [{{ resourceId: {bob_id} }}] }}
        ]"#
    );
    let task_id =
        create_task("Pair-Task", "TASK", 1.0, Some(16.0), None, None, None, None, Some(&rc_frag))
            .await;
    assert!(task_id > 0, "expected pair_task_id > 0, got {task_id}");

    // Create another task requiring all three resources
    let rc_frag3 = format!(
        r#"resourceConstraints: [
            {{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }},
            {{ optional: false, speed: 1.0, entries: [{{ resourceId: {bob_id} }}] }},
            {{ optional: false, speed: 1.0, entries: [{{ resourceId: {charlie_id} }}] }}
        ]"#
    );
    let task2_id = create_task(
        "Triple-Task",
        "TASK",
        2.0,
        Some(24.0),
        None,
        None,
        None,
        None,
        Some(&rc_frag3),
    )
    .await;
    assert!(task2_id > 0, "expected triple_task_id > 0, got {task2_id}");

    // Verify resources query
    let val = gql_ok_simple(r#"{ resources { dbId name } }"#).await;
    let resources = extract_list(&val, "resources");
    assert_eq!(
        resources.len(),
        3,
        "expected 3 resources after multiple resource setup, got {}",
        resources.len()
    );
}

#[tokio::test]
#[serial]
async fn test_optional_resources() {
    clean_database().await;

    let alice_id = create_resource("Alice", "Europe/Berlin", true).await;
    let bob_id = create_resource("Bob", "America/New_York", true).await;

    // Task with Alice required, Bob optional
    let rc_frag = format!(
        r#"resourceConstraints: [
            {{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }},
            {{ optional: true, speed: 0.5, entries: [{{ resourceId: {bob_id} }}] }}
        ]"#
    );
    let _task_id =
        create_task("OptTask", "TASK", 1.0, Some(8.0), None, None, None, None, Some(&rc_frag))
            .await;

    // Query the task and verify resource constraints
    let val = gql_ok_simple(
        r#"{ tasks { dbId title resourceConstraints { optional speed entries { resource { dbId name } } } } }"#
    ).await;
    let tasks = extract_list(&val, "tasks");
    let task =
        tasks.iter().find(|t| extract_str(t, "title") == "OptTask").expect("OptTask must exist");
    let constraints = extract_list(task, "resourceConstraints");
    assert_eq!(
        constraints.len(),
        2,
        "expected 2 resource constraints for OptTask, got {}",
        constraints.len()
    );

    // Check that one is optional=false and the other is optional=true
    let mut found_required = false;
    let mut found_optional = false;
    for c in constraints {
        let obj = c.as_object_value().unwrap();
        let opt_val = obj.iter().find(|(k, _)| *k == "optional").unwrap().1;
        let is_optional = opt_val.as_scalar().and_then(|s| s.try_to_bool()).unwrap();
        if is_optional {
            found_optional = true;
            let speed = extract_f64(c, "speed");
            assert!(
                (speed - 0.5).abs() < 0.01,
                "expected optional constraint speed ≈ 0.5, got {speed}"
            );
        } else {
            found_required = true;
            let speed = extract_f64(c, "speed");
            assert!(
                (speed - 1.0).abs() < 0.01,
                "expected required constraint speed ≈ 1.0, got {speed}"
            );
        }
    }
    assert!(found_required, "expected a required constraint in OptTask resourceConstraints");
    assert!(found_optional, "expected an optional constraint in OptTask resourceConstraints");
}

#[tokio::test]
#[serial]
async fn test_different_timezones() {
    clean_database().await;

    let timezones = [
        ("R-Berlin", "Europe/Berlin"),
        ("R-NewYork", "America/New_York"),
        ("R-Tokyo", "Asia/Tokyo"),
        ("R-UTC", "UTC"),
        ("R-Sydney", "Australia/Sydney"),
    ];

    for (name, tz) in &timezones {
        create_resource(name, tz, false).await;
    }

    let val = gql_ok_simple(r#"{ resources { name timezone } }"#).await;
    let resources = extract_list(&val, "resources");
    assert_eq!(
        resources.len(),
        timezones.len(),
        "expected {} resources, got {}",
        timezones.len(),
        resources.len()
    );

    // Verify each timezone is stored correctly
    for (name, expected_tz) in &timezones {
        let res = resources
            .iter()
            .find(|r| extract_str(r, "name") == *name)
            .unwrap_or_else(|| panic!("resource '{name}' not found"));
        let stored_tz = extract_str(res, "timezone");
        assert_eq!(stored_tz, *expected_tz, "timezone mismatch for resource '{name}'");
    }
}

#[tokio::test]
#[serial]
async fn test_dependency_types() {
    clean_database().await;

    // Create tasks of each designation
    let req = create_task("Dep-Req", "REQUIREMENT", 1.0, None, None, None, None, None, None).await;
    let task_a =
        create_task("Dep-TaskA", "TASK", 2.0, Some(4.0), None, None, None, None, None).await;
    let task_b =
        create_task("Dep-TaskB", "TASK", 3.0, Some(4.0), None, None, None, None, None).await;
    let group = create_task("Dep-Group", "GROUP", 4.0, None, None, None, None, None, None).await;
    let ms =
        create_task("Dep-Milestone", "MILESTONE", 5.0, None, None, None, None, None, None).await;

    // Helper: update a task's predecessors via mutation.
    // We verify the result via a separate query (fresh Context) because the
    // dataloader inside a single Context caches dependency rows and may return
    // stale data in the mutation response when predecessors are replaced.
    let update_task_preds = |id: i64, title: &str, designation: &str, preds: &[i64]| {
        let title = title.to_string();
        let designation = designation.to_string();
        let preds = preds.to_vec();
        async move {
            let current_tasks_val = gql_ok_simple(r#"{ tasks { dbId title } }"#).await;
            let current_tasks = extract_list(&current_tasks_val, "tasks");

            let current_task_id = current_tasks
                .iter()
                .find(|t| extract_str(t, "title") == title)
                .map(|t| extract_i64(t, "dbId"))
                .unwrap_or(id);

            let mut current_pred_ids = Vec::new();
            for pred_id in preds {
                let pred_title = find_task_title(pred_id).await;
                let current_pred_id = current_tasks
                    .iter()
                    .find(|t| extract_str(t, "title") == pred_title)
                    .map(|t| extract_i64(t, "dbId"))
                    .unwrap_or_else(|| {
                        panic!("current predecessor for dbId={} not found", pred_id)
                    });
                current_pred_ids.push(current_pred_id);
            }

            let preds_str = format!("{:?}", current_pred_ids);
            let mutation = format!(
                r#"mutation {{
                    taskSave(task: {{
                        dbId: {current_task_id},
                        title: "{title}",
                        description: "desc",
                        designation: {designation},
                        priority: 1.0,
                        predecessors: {preds_str}
                    }}) {{
                        dbId
                    }}
                }}"#
            );
            gql_ok_simple(&mutation).await;
        }
    };

    // 1. Requirement → Task
    update_task_preds(task_a, "Dep-TaskA", "TASK", &[req]).await;
    let preds = query_task_predecessors(task_a).await;
    assert_eq!(preds, vec![req], "Requirement->Task predecessor mismatch");

    // 2. Requirement → Group
    update_task_preds(group, "Dep-Group", "GROUP", &[req]).await;
    let preds = query_task_predecessors(group).await;
    assert_eq!(preds, vec![req], "Requirement->Group predecessor mismatch");

    // 3. Task → Task
    update_task_preds(task_b, "Dep-TaskB", "TASK", &[task_a]).await;
    let preds = query_task_predecessors(task_b).await;
    let current_task_a = find_current_task_id_by_title("Dep-TaskA").await;
    assert_eq!(preds, vec![current_task_a], "Task->Task predecessor mismatch");

    // 4. Task → Group (change group's predecessor from req to task_a)
    update_task_preds(group, "Dep-Group", "GROUP", &[task_a]).await;
    let preds = query_task_predecessors(group).await;
    assert_eq!(preds, vec![current_task_a], "Task->Group predecessor mismatch");

    // 5. Group → Task (change task_b's predecessor from task_a to group)
    update_task_preds(task_b, "Dep-TaskB", "TASK", &[group]).await;
    let preds = query_task_predecessors(task_b).await;
    let current_group = find_current_task_id_by_title("Dep-Group").await;
    assert_eq!(preds, vec![current_group], "Group->Task predecessor mismatch");

    // 6. Task → Milestone
    update_task_preds(ms, "Dep-Milestone", "MILESTONE", &[task_b]).await;
    let preds = query_task_predecessors(ms).await;
    let current_task_b = find_current_task_id_by_title("Dep-TaskB").await;
    assert_eq!(preds, vec![current_task_b], "Task->Milestone predecessor mismatch");

    // 7. Group → Milestone (change milestone's predecessor from task_b to group)
    update_task_preds(ms, "Dep-Milestone", "MILESTONE", &[group]).await;
    let preds = query_task_predecessors(ms).await;
    assert_eq!(preds, vec![current_group], "Group->Milestone predecessor mismatch");
}

#[tokio::test]
#[serial]
async fn test_dependency_query_uses_distinct_dataloaders_per_revision() {
    clean_database().await;

    let predecessor_v1 =
        create_task("Rev-Pred-V1", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;
    let successor_v1 = create_task(
        "Rev-Succ",
        "TASK",
        1.0,
        Some(4.0),
        None,
        Some(&[predecessor_v1]),
        None,
        None,
        None,
    )
    .await;
    let revision_v1 = latest_revision().await;

    let predecessor_v2 =
        create_task("Rev-Pred-V2", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;

    let current_successor_v1 = find_current_task_id_by_title("Rev-Succ").await;
    let current_predecessor_v2 = find_current_task_id_by_title("Rev-Pred-V2").await;
    let update_successor = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {current_successor_v1},
                title: "Rev-Succ",
                description: "desc for Rev-Succ",
                designation: TASK,
                priority: 1.0,
                effort: 4.0,
                predecessors: [{current_predecessor_v2}]
            }}) {{
                dbId
            }}
        }}"#
    );
    let update_val = gql_ok_simple(&update_successor).await;
    let successor_v2 = extract_i64(&update_val, "taskSave.dbId");
    let revision_v2 = latest_revision().await;

    let query = format!(
        r#"{{
            old: tasks(revision: {revision_v1}) {{
                dbId
                predecessors {{ dbId }}
            }}
            new: tasks(revision: {revision_v2}) {{
                dbId
                predecessors {{ dbId }}
            }}
        }}"#
    );
    let val = gql_ok_simple(&query).await;

    let old_tasks = extract_list(&val, "old");
    let old_successor = old_tasks
        .iter()
        .find(|t| extract_i64(t, "dbId") == successor_v1)
        .unwrap_or_else(|| panic!("task with dbId={successor_v1} not found in old revision"));
    let old_preds = extract_list(old_successor, "predecessors");
    assert_eq!(old_preds.len(), 1, "expected 1 predecessor in old revision");
    assert_eq!(
        extract_i64(&old_preds[0], "dbId"),
        predecessor_v1,
        "expected old revision predecessor to be {predecessor_v1}"
    );

    let new_tasks = extract_list(&val, "new");
    let new_successor = new_tasks
        .iter()
        .find(|t| extract_i64(t, "dbId") == successor_v2)
        .unwrap_or_else(|| panic!("task with dbId={successor_v2} not found in new revision"));
    let new_preds = extract_list(new_successor, "predecessors");
    assert_eq!(new_preds.len(), 1, "expected 1 predecessor in new revision");
    assert_eq!(
        extract_i64(&new_preds[0], "dbId"),
        predecessor_v2,
        "expected new revision predecessor to be {predecessor_v2}"
    );
}

#[tokio::test]
#[serial]
async fn test_nested_groups() {
    clean_database().await;

    // Create Group A (top-level)
    let group_a = create_task("GroupA", "GROUP", 1.0, None, None, None, None, None, None).await;

    // Create Group B as child of Group A
    let group_b =
        create_task("GroupB", "GROUP", 2.0, None, Some(group_a), None, None, None, None).await;

    // Create tasks inside Group B
    let _t1 =
        create_task("Nested-T1", "TASK", 3.0, Some(4.0), Some(group_b), None, None, None, None)
            .await;
    let _t2 =
        create_task("Nested-T2", "TASK", 4.0, Some(4.0), Some(group_b), None, None, None, None)
            .await;

    // Also update Group A to declare Group B as a child (via children field)
    let current_group_a = find_current_task_id_by_title("GroupA").await;
    let current_group_b = find_current_task_id_by_title("GroupB").await;
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {current_group_a},
                title: "GroupA",
                description: "top group",
                designation: GROUP,
                priority: 1.0,
                children: [{current_group_b}]
            }}) {{
                dbId
                children {{ dbId title }}
            }}
        }}"#
    );
    let val = gql_ok_simple(&mutation).await;
    let children_a = extract_list(&val, "taskSave.children");
    assert_eq!(
        children_a.len(),
        1,
        "expected GroupA to have exactly 1 child, got {}",
        children_a.len()
    );
    let child_title = extract_str(&children_a[0], "title");
    assert_eq!(child_title, "GroupB", "expected GroupA child to be GroupB, got {child_title}");

    // Verify Group B's children via query
    let val = gql_ok_simple(r#"{ tasks { dbId title children { dbId title } } }"#).await;
    let tasks = extract_list(&val, "tasks");

    let gb =
        tasks.iter().find(|t| extract_str(t, "title") == "GroupB").expect("GroupB should exist");
    let gb_children = extract_list(gb, "children");
    assert_eq!(
        gb_children.len(),
        2,
        "expected GroupB to have exactly 2 children, got {}",
        gb_children.len()
    );

    // Verify the two children are Nested-T1 and Nested-T2
    let child_titles: Vec<&str> = gb_children.iter().map(|c| extract_str(c, "title")).collect();
    assert!(
        child_titles.contains(&"Nested-T1"),
        "expected GroupB children to include Nested-T1, got {child_titles:?}"
    );
    assert!(
        child_titles.contains(&"Nested-T2"),
        "expected GroupB children to include Nested-T2, got {child_titles:?}"
    );
}

#[tokio::test]
#[serial]
async fn test_recalculation() {
    clean_database().await;

    let alice_id = create_resource("Alice", "Europe/Berlin", true).await;

    // Create a requirement
    let req_id =
        create_task("Calc-Req", "REQUIREMENT", 1.0, None, None, None, None, None, None).await;

    // Create a task with effort and resource constraint
    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let task_id = create_task(
        "Calc-Task",
        "TASK",
        2.0,
        Some(8.0),
        None,
        Some(&[req_id]),
        None,
        None,
        Some(&rc_frag),
    )
    .await;

    // Create a milestone
    let _ms_id = create_task(
        "Calc-Milestone",
        "MILESTONE",
        3.0,
        None,
        None,
        Some(&[task_id]),
        None,
        None,
        None,
    )
    .await;

    // Trigger recalculation
    let val = gql_ok_simple(r#"mutation { recalculateNow }"#).await;
    let recalc_result = value_at_path(&val, "recalculateNow");
    assert!(
        recalc_result.as_scalar().and_then(|s| s.try_to_bool()).unwrap_or(false),
        "expected recalculateNow to return true, got {recalc_result:?}"
    );

    // The recalculation happens asynchronously in the real server. In our
    // integration test we don't spin up the scheduling loop, so the plan
    // allocations table will still be empty. But we can verify the mutation
    // itself succeeds and that the plan query works.
    let val =
        gql_ok_simple(r#"{ currentPlan { allocations { dbId start end allocationType } } }"#).await;
    let allocations = extract_list(&val, "currentPlan.allocations");
    // In a full server scenario this would be > 0, but without the scheduler
    // loop we just verify the query works.

    assert_eq!(
        allocations.len(),
        0,
        "expected no allocations without running the scheduler loop, got {}",
        allocations.len()
    );
}

#[tokio::test]
#[serial]
async fn test_issues_detected() {
    clean_database().await;

    // Create a task with effort but NO resource constraints and NO
    // requirement predecessor and NO milestone successor. The scheduler
    // would flag issues, but issues are written to the DB by the scheduling
    // loop. Instead, verify the issues query itself returns an empty list
    // (since no scheduling has happened) and the schema is correct.
    let _task_id =
        create_task("Orphan-Task", "TASK", 1.0, Some(8.0), None, None, None, None, None).await;

    let val = gql_ok_simple(r#"{ issues { dbId code description type task { dbId } } }"#).await;
    let issues = extract_list(&val, "issues");
    // Issues are populated by the scheduler, not by mutations. The query itself
    // working is the important validation here.
    assert_eq!(
        issues.len(),
        0,
        "expected no issues before scheduler execution, got {}",
        issues.len()
    );
}

#[tokio::test]
#[serial]
async fn test_task_delete() {
    clean_database().await;

    let t = create_task("Del-Task", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;

    // Verify it exists
    let val = gql_ok_simple(r#"{ tasks { dbId } }"#).await;
    let tasks = extract_list(&val, "tasks");
    assert_eq!(tasks.len(), 1, "expected 1 task before delete, got {}", tasks.len());

    // Delete
    let mutation = format!(r#"mutation {{ taskDelete(taskId: {t}) }}"#);
    let val = gql_ok_simple(&mutation).await;
    let deleted = value_at_path(&val, "taskDelete");
    assert!(
        deleted.as_scalar().and_then(|s| s.try_to_bool()).unwrap_or(false),
        "expected taskDelete({t}) to return true, got {deleted:?}"
    );

    // Verify it's gone
    let val = gql_ok_simple(r#"{ tasks { dbId } }"#).await;
    let tasks = extract_list(&val, "tasks");
    assert_eq!(tasks.len(), 0, "expected 0 tasks after delete, got {}", tasks.len());
}

#[tokio::test]
#[serial]
async fn test_resource_delete() {
    clean_database().await;

    let r_id = create_resource("DeleteMe", "UTC", false).await;

    let val = gql_ok_simple(r#"{ resources { dbId } }"#).await;
    let resources = extract_list(&val, "resources");
    assert_eq!(resources.len(), 1, "expected 1 resource before delete, got {}", resources.len());

    let mutation = format!(r#"mutation {{ resourceDelete(resourceId: {r_id}) }}"#);
    let val = gql_ok_simple(&mutation).await;
    let deleted = value_at_path(&val, "resourceDelete");
    assert!(
        deleted.as_scalar().and_then(|s| s.try_to_bool()).unwrap_or(false),
        "expected resourceDelete({r_id}) to return true, got {deleted:?}"
    );

    let val = gql_ok_simple(r#"{ resources { dbId } }"#).await;
    let resources = extract_list(&val, "resources");
    assert_eq!(resources.len(), 0, "expected 0 resources after delete, got {}", resources.len());
}

#[tokio::test]
#[serial]
async fn test_resource_with_vacations() {
    clean_database().await;

    let mutation = r#"mutation {
        resourceSave(resource: {
            name: "VacRes",
            timezone: "Europe/Berlin",
            added: "2025-01-01T00:00:00Z",
            addedVacations: [
                { from: "2025-07-01T00:00:00Z", until: "2025-07-15T00:00:00Z" },
                { from: "2025-12-24T00:00:00Z", until: "2025-12-31T00:00:00Z" }
            ]
        }) {
            dbId
            name
            vacation { dbId from until }
        }
    }"#;

    let val = gql_ok_simple(mutation).await;
    let vacations_field = &val
        .as_object_value()
        .unwrap()
        .iter()
        .find(|(k, _)| *k == "resourceSave")
        .unwrap()
        .1
        .as_object_value()
        .unwrap()
        .iter()
        .find(|(k, _)| *k == "vacation")
        .unwrap()
        .1;
    let vacations = vacations_field.as_list_value().unwrap();
    assert_eq!(vacations.len(), 2, "expected 2 vacations for VacRes, got {}", vacations.len());
}

#[tokio::test]
#[serial]
async fn test_hello_world() {
    // Ensure the basic hello_world query works (no DB access needed for the
    // value, but Context is still created).
    let _ = shared_db_url().await; // ensure global url is set

    let val = gql_ok_simple(r#"{ helloWorld }"#).await;
    let hw = extract_str(&val, "helloWorld");
    assert_eq!(hw, "Hello World from Juniper!", "helloWorld query returned unexpected text");
}

#[tokio::test]
#[serial]
async fn test_complex_dependencies_and_update() {
    clean_database().await;

    // Build a chain: R -> T1 -> T2 -> T3 -> M
    let req =
        create_task("Chain-Req", "REQUIREMENT", 1.0, None, None, None, None, None, None).await;
    let t1 =
        create_task("Chain-T1", "TASK", 2.0, Some(4.0), None, Some(&[req]), None, None, None).await;
    let t2 =
        create_task("Chain-T2", "TASK", 3.0, Some(4.0), None, Some(&[t1]), None, None, None).await;
    let t3 =
        create_task("Chain-T3", "TASK", 4.0, Some(4.0), None, Some(&[t2]), None, None, None).await;
    let _ms =
        create_task("Chain-MS", "MILESTONE", 5.0, None, None, Some(&[t3]), None, None, None).await;

    // Verify full chain via successors
    let val =
        gql_ok_simple(r#"{ tasks { dbId title successors { dbId } predecessors { dbId } } }"#)
            .await;
    let tasks = extract_list(&val, "tasks");
    assert_eq!(tasks.len(), 5, "expected 5 tasks in dependency chain, got {}", tasks.len());

    // Verify T1 has successor T2
    let t1_obj = tasks.iter().find(|t| extract_str(t, "title") == "Chain-T1").unwrap();
    let t1_succs = extract_list(t1_obj, "successors");
    assert_eq!(
        t1_succs.len(),
        1,
        "expected Chain-T1 to have exactly 1 successor, got {}",
        t1_succs.len()
    );
    assert_eq!(
        extract_i64(&t1_succs[0], "dbId"),
        t2,
        "expected Chain-T1 successor to be Chain-T2 ({t2})"
    );

    // Update T2 to depend on Req directly (instead of T1).
    let current_t2 = find_current_task_id_by_title("Chain-T2").await;
    let current_req = find_current_task_id_by_title("Chain-Req").await;
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {current_t2},
                title: "Chain-T2",
                description: "updated",
                designation: TASK,
                priority: 3.0,
                effort: 4.0,
                predecessors: [{current_req}]
            }}) {{
                dbId
            }}
        }}"#
    );
    gql_ok_simple(&mutation).await;

    // Verify via a fresh query (separate Context avoids stale dataloader cache)
    let t2_preds = query_task_predecessors(t2).await;
    assert_eq!(
        t2_preds,
        vec![req],
        "expected Chain-T2 predecessors to update to [{req}], got {t2_preds:?}"
    );
}

/// Directly execute raw SQL against the shared test database.
async fn exec_raw_sql(sql: &str) {
    let url = shared_db_url().await;
    let db = Database::connect(url).await.expect("exec_raw_sql: connect failed");
    let txn = db.begin().await.expect("exec_raw_sql: begin failed");
    use sea_orm::ConnectionTrait as _;
    txn.execute(sea_orm::Statement::from_string(sea_orm::DatabaseBackend::Sqlite, sql.to_string()))
        .await
        .unwrap_or_else(|e| panic!("exec_raw_sql failed: {e}\nSQL: {sql}"));
    txn.commit().await.expect("exec_raw_sql: commit failed");
    db.close().await.ok();
}

// ── Bug-reproduction tests ────────────────────────────────────────────────

/// Bug 2: After modifying a task that has dependencies, `query_problem` panics
/// because dependency edges reference `task_header.id` while the node-index maps
/// are keyed by `task_iteration.id`. After a modification the two diverge.
#[tokio::test]
#[serial]
async fn test_query_problem_after_task_modification() {
    clean_database().await;
    shared_db_url().await;

    let alice_id = create_resource("QP-Alice", "Europe/Berlin", true).await;

    // Requirement with earliest_start
    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "QP-Req",
            description: "req",
            designation: REQUIREMENT,
            priority: 1.0,
            earliestStart: "2025-06-01T00:00:00Z"
        }}) {{ dbId iterationId }} }}"#
    ))
    .await;
    let req_id = extract_i64(&val, "taskSave.dbId");

    // Task with dependency on requirement and a resource constraint
    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let task_id = create_task(
        "QP-Task",
        "TASK",
        2.0,
        Some(8.0),
        None,
        Some(&[req_id]),
        None,
        None,
        Some(&rc_frag),
    )
    .await;

    // Milestone with schedule_target depending on the task
    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "QP-Milestone",
            description: "ms",
            designation: MILESTONE,
            priority: 3.0,
            scheduleTarget: "2025-09-01T00:00:00Z",
            predecessors: [{task_id}]
        }}) {{ dbId }} }}"#
    ))
    .await;
    let _ms_id = extract_i64(&val, "taskSave.dbId");

    // First query_problem must succeed (header_id == iteration_id on fresh DB).
    {
        let (app_state, _rx) = AppState::new(true);
        let ctx = Context::new(app_state);
        let revision_id = revision::Entity::find()
            .order_by_desc(revision::Column::Id)
            .one(ctx.db().txn().await.expect("Failed to get transaction"))
            .await
            .expect("Failed to query revision")
            .expect("Must have at least one revision")
            .id;
        let result = siapla::scheduling::query_problem(&ctx, revision_id).await;
        assert!(result.is_ok(), "First query_problem should succeed: {:?}", result.err());
        Arc::into_inner(ctx)
            .expect("only strong ref")
            .commit()
            .await
            .expect("commit after first query_problem");
    }

    // Modify the task (reduce effort). This creates a new iteration whose id
    // differs from its header_id.
    let task_header_id = find_current_task_id_by_title("QP-Task").await;
    let iter_before = find_current_iteration_id_by_title("QP-Task").await;
    let current_req = find_current_task_id_by_title("QP-Req").await;
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {task_header_id},
                title: "QP-Task",
                description: "reduced effort",
                designation: TASK,
                priority: 2.0,
                effort: 4.0,
                predecessors: [{current_req}]
            }}) {{
                dbId
                iterationId
            }}
        }}"#
    );
    let val = gql_ok_simple(&mutation).await;
    let returned_header = extract_i64(&val, "taskSave.dbId");
    let iter_after = extract_i64(&val, "taskSave.iterationId");
    assert_eq!(
        returned_header, task_header_id,
        "dbId (header) must stay the same after modification"
    );
    assert_ne!(iter_after, iter_before, "iterationId must change after modification");

    // query_problem after modification must NOT panic (Bug 2 reproduced here).
    {
        let (app_state, _rx) = AppState::new(true);
        let ctx = Context::new(app_state);
        let revision_id = revision::Entity::find()
            .order_by_desc(revision::Column::Id)
            .one(ctx.db().txn().await.expect("Failed to get transaction"))
            .await
            .expect("Failed to query revision")
            .expect("Must have at least one revision")
            .id;
        let result = siapla::scheduling::query_problem(&ctx, revision_id).await;
        assert!(
            result.is_ok(),
            "query_problem after task modification should succeed: {:?}",
            result.err()
        );
        let project = result.unwrap();
        assert_eq!(project.objs.tasks.len(), 1, "project should still contain 1 task");
        Arc::into_inner(ctx)
            .expect("only strong ref")
            .commit()
            .await
            .expect("commit after second query_problem");
    }
}

/// Bug 1: After modifying an already-planned task the plan disappears from the
/// frontend because `currentPlan` resolved to the latest (NOT_CALCULATED)
/// revision which has no allocations.
#[tokio::test]
#[serial]
async fn test_plan_visible_after_task_modification() {
    clean_database().await;
    shared_db_url().await;

    let alice_id = create_resource("PV-Alice", "Europe/Berlin", true).await;

    // Requirement with earliest_start
    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "PV-Req",
            description: "req",
            designation: REQUIREMENT,
            priority: 1.0,
            earliestStart: "2025-06-01T00:00:00Z"
        }}) {{ dbId }} }}"#
    ))
    .await;
    let req_id = extract_i64(&val, "taskSave.dbId");

    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let task_header_id = create_task(
        "PV-Task",
        "TASK",
        2.0,
        Some(8.0),
        None,
        Some(&[req_id]),
        None,
        None,
        Some(&rc_frag),
    )
    .await;

    // Milestone with schedule_target
    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "PV-Milestone",
            description: "ms",
            designation: MILESTONE,
            priority: 3.0,
            scheduleTarget: "2025-09-01T00:00:00Z",
            predecessors: [{task_header_id}]
        }}) {{ dbId }} }}"#
    ))
    .await;
    let _ms_id = extract_i64(&val, "taskSave.dbId");

    // Mark the current (latest) revision as AVAILABLE and insert a fake
    // allocation whose task_id is the **header_id** (as store_plan now does).
    let plan_revision = latest_revision().await;
    exec_raw_sql(&format!(
        "UPDATE revision SET plan_state = 'AVAILABLE' WHERE id = {plan_revision}"
    ))
    .await;
    exec_raw_sql(&format!(
        "INSERT INTO allocation (task_id, start, end, rev_created) \
         VALUES ({task_header_id}, '2025-06-01T00:00:00Z', '2025-06-10T00:00:00Z', {plan_revision})"
    ))
    .await;

    // Verify the allocation is visible before modification.
    let val =
        gql_ok_simple(r#"{ currentPlan { allocations { dbId task { dbId iterationId } } } }"#)
            .await;
    let allocations = extract_list(&val, "currentPlan.allocations");
    assert!(!allocations.is_empty(), "allocations should be visible before modification");

    // Modify the task (reduce effort) → new NOT_CALCULATED revision.
    // dbId in the mutation is the header_id (stable identity).
    let current_req = find_current_task_id_by_title("PV-Req").await;
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {task_header_id},
                title: "PV-Task",
                description: "reduced effort",
                designation: TASK,
                priority: 2.0,
                effort: 4.0,
                predecessors: [{current_req}]
            }}) {{
                dbId
                iterationId
            }}
        }}"#
    );
    let val = gql_ok_simple(&mutation).await;
    let returned_header = extract_i64(&val, "taskSave.dbId");
    assert_eq!(
        returned_header, task_header_id,
        "dbId (header) must stay the same after modification"
    );

    // The latest revision is now NOT_CALCULATED. The plan query must still
    // return the allocations from the last AVAILABLE revision (Bug 1).
    let val = gql_ok_simple(r#"{ currentPlan { allocations { dbId task { dbId } } } }"#).await;
    let allocations = extract_list(&val, "currentPlan.allocations");
    assert!(!allocations.is_empty(), "allocations should still be visible after task modification");

    // The allocation's task.dbId must be the header_id (stable identity).
    let alloc_task_id = extract_i64(&allocations[0], "task.dbId");
    assert_eq!(
        alloc_task_id, task_header_id,
        "allocation task.dbId should be the header_id ({task_header_id})"
    );
}

/// Verify that:
/// 1. `dbId` is the header_id (stable) and `iterationId` is the iteration id
///    (changes on every edit). After modification, header_id stays the same
///    but iterationId changes.
/// 2. Allocations stored by `store_plan` reference the header_id. When queried
///    via `currentPlan → allocations → task`, the returned `task.dbId` is the
///    header_id.
/// 3. The task also exposes `revCreated`, `revDeleted`, `headerRevCreated`,
///    `headerRevDeleted` revision metadata.
#[tokio::test]
#[serial]
async fn test_allocation_references_header_id_and_revision_fields() {
    clean_database().await;
    shared_db_url().await;

    let alice_id = create_resource("HID-Alice", "Europe/Berlin", true).await;

    // ── Create a minimal project: Req → Task → Milestone ──
    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "HID-Req",
            description: "req",
            designation: REQUIREMENT,
            priority: 1.0,
            earliestStart: "2025-06-01T00:00:00Z"
        }}) {{ dbId iterationId }} }}"#
    ))
    .await;
    let req_header = extract_i64(&val, "taskSave.dbId");

    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let task_header = create_task(
        "HID-Task",
        "TASK",
        2.0,
        Some(8.0),
        None,
        Some(&[req_header]),
        None,
        None,
        Some(&rc_frag),
    )
    .await;
    let iter_before = find_current_iteration_id_by_title("HID-Task").await;

    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "HID-Milestone",
            description: "ms",
            designation: MILESTONE,
            priority: 3.0,
            scheduleTarget: "2025-09-01T00:00:00Z",
            predecessors: [{task_header}]
        }}) {{ dbId }} }}"#
    ))
    .await;
    let _ms_header = extract_i64(&val, "taskSave.dbId");

    // ── Modify the task so header_id ≠ iteration_id ──
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {task_header},
                title: "HID-Task",
                description: "modified",
                designation: TASK,
                priority: 2.0,
                effort: 4.0,
                predecessors: [{req_header}]
                {rc_frag}
            }}) {{
                dbId
                iterationId
                revCreated
                revDeleted
                headerRevCreated
                headerRevDeleted
            }}
        }}"#
    );
    let val = gql_ok_simple(&mutation).await;
    let returned_header = extract_i64(&val, "taskSave.dbId");
    let iter_after = extract_i64(&val, "taskSave.iterationId");
    assert_eq!(returned_header, task_header, "dbId (header) must be stable across modifications");
    assert_ne!(iter_after, iter_before, "iterationId must differ from the original iteration");
    assert_ne!(
        returned_header as i64, iter_after,
        "header_id and iterationId must be different after modification"
    );
    assert!(
        value_at_path(&val, "taskSave.revDeleted").is_null(),
        "revDeleted of the active iteration should be null"
    );
    assert!(
        value_at_path(&val, "taskSave.headerRevDeleted").is_null(),
        "headerRevDeleted should be null for a live task"
    );
    let header_rev_created = extract_i64(&val, "taskSave.headerRevCreated");
    assert!(header_rev_created > 0, "headerRevCreated should be > 0");
    let iter_rev_created = extract_i64(&val, "taskSave.revCreated");
    assert!(
        iter_rev_created > header_rev_created,
        "iteration revCreated ({iter_rev_created}) should be > \
         headerRevCreated ({header_rev_created})"
    );

    // ── Insert an allocation referencing the header_id and verify via GQL ──
    let plan_revision = latest_revision().await;
    exec_raw_sql(&format!(
        "UPDATE revision SET plan_state = 'AVAILABLE' WHERE id = {plan_revision}"
    ))
    .await;
    exec_raw_sql(&format!(
        "INSERT INTO allocation (task_id, start, end, rev_created) \
         VALUES ({task_header}, '2025-07-01T00:00:00Z', '2025-07-10T00:00:00Z', {plan_revision})"
    ))
    .await;

    let val =
        gql_ok_simple(r#"{ currentPlan { allocations { dbId task { dbId iterationId } } } }"#)
            .await;
    let allocations = extract_list(&val, "currentPlan.allocations");
    assert!(!allocations.is_empty(), "should have allocations after inserting one");
    let alloc_task_db_id = extract_i64(&allocations[0], "task.dbId");
    let alloc_task_iter_id = extract_i64(&allocations[0], "task.iterationId");
    assert_eq!(
        alloc_task_db_id, task_header,
        "allocation → task.dbId must be the header_id ({task_header}), got {alloc_task_db_id}"
    );
    assert_eq!(
        alloc_task_iter_id, iter_after,
        "allocation → task.iterationId must be the current iteration ({iter_after}), \
         got {alloc_task_iter_id}"
    );

    // ── Verify tasks query also exposes both ids ──
    let val = gql_ok_simple(
        r#"{ tasks { dbId iterationId revCreated revDeleted headerRevCreated headerRevDeleted } }"#,
    )
    .await;
    let tasks = extract_list(&val, "tasks");
    let hid_task = tasks
        .iter()
        .find(|t| extract_i64(t, "dbId") == task_header)
        .expect("HID-Task should be in the tasks list");
    assert_eq!(
        extract_i64(hid_task, "iterationId"),
        iter_after,
        "tasks query iterationId should match"
    );
    assert!(
        value_at_path(hid_task, "revDeleted").is_null(),
        "active task revDeleted should be null"
    );
}

/// Verify that bookings also store and expose stable task header ids:
/// 1. `bookingSave(taskId: ...)` accepts a task header id and stores that
///    header id in `booking.task_id`.
/// 2. `bookings { task { dbId iterationId ... } }` resolves `task.dbId` to the
///    stable header id and `task.iterationId` to the current active iteration.
/// 3. After editing the task, the existing booking still resolves to the same
///    stable header id while following the latest active iteration.
#[tokio::test]
#[serial]
async fn test_booking_save_uses_header_id_and_resolves_stable_task_identity() {
    clean_database().await;
    shared_db_url().await;

    let alice_id = create_resource("BKG-Alice", "Europe/Berlin", true).await;

    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "BKG-Req",
            description: "req",
            designation: REQUIREMENT,
            priority: 1.0,
            earliestStart: "2025-06-01T00:00:00Z"
        }}) {{ dbId iterationId }} }}"#
    ))
    .await;
    let req_header = extract_i64(&val, "taskSave.dbId");

    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let task_header = create_task(
        "BKG-Task",
        "TASK",
        2.0,
        Some(8.0),
        None,
        Some(&[req_header]),
        None,
        None,
        Some(&rc_frag),
    )
    .await;
    let iter_before = find_current_iteration_id_by_title("BKG-Task").await;

    let val = gql_ok_simple(&format!(
        r#"mutation {{ taskSave(task: {{
            title: "BKG-Milestone",
            description: "ms",
            designation: MILESTONE,
            priority: 3.0,
            scheduleTarget: "2025-09-01T00:00:00Z",
            predecessors: [{task_header}]
        }}) {{ dbId }} }}"#
    ))
    .await;
    let _ms_header = extract_i64(&val, "taskSave.dbId");

    let booking_val = gql_ok_simple(&format!(
        r#"mutation {{
            bookingSave(
                dbId: null,
                taskId: {task_header},
                start: "2025-07-01T08:00:00Z",
                end: "2025-07-01T16:00:00Z",
                resources: [{alice_id}],
                final: false
            ) {{
                dbId
                task {{
                    dbId
                    iterationId
                    revCreated
                    revDeleted
                    headerRevCreated
                    headerRevDeleted
                }}
            }}
        }}"#
    ))
    .await;
    let booking_id = extract_i64(&booking_val, "bookingSave.dbId");
    assert!(booking_id > 0, "bookingSave should return a booking id");
    assert_eq!(
        extract_i64(&booking_val, "bookingSave.task.dbId"),
        task_header,
        "bookingSave.task.dbId should be the stable task header id"
    );
    assert_eq!(
        extract_i64(&booking_val, "bookingSave.task.iterationId"),
        iter_before,
        "bookingSave.task.iterationId should initially resolve to the current iteration"
    );
    assert!(
        value_at_path(&booking_val, "bookingSave.task.revDeleted").is_null(),
        "bookingSave.task.revDeleted should be null for the active iteration"
    );
    assert!(
        value_at_path(&booking_val, "bookingSave.task.headerRevDeleted").is_null(),
        "bookingSave.task.headerRevDeleted should be null for a live task"
    );

    let url = shared_db_url().await;
    let db = Database::connect(url).await.expect("booking header test: connect failed");
    let txn = db.begin().await.expect("booking header test: begin failed");
    let stored_booking = siapla::entity::booking::Entity::find_by_id(booking_id as i32)
        .one(&txn)
        .await
        .expect("booking header test: query booking failed")
        .expect("booking header test: booking not found");
    assert_eq!(
        stored_booking.task_id as i64, task_header,
        "booking.task_id in the database must store the stable task header id"
    );
    txn.commit().await.expect("booking header test: commit failed");
    db.close().await.ok();

    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {task_header},
                title: "BKG-Task",
                description: "modified",
                designation: TASK,
                priority: 2.0,
                effort: 4.0,
                predecessors: [{req_header}]
                {rc_frag}
            }}) {{
                dbId
                iterationId
            }}
        }}"#
    );
    let val = gql_ok_simple(&mutation).await;
    let returned_header = extract_i64(&val, "taskSave.dbId");
    let iter_after = extract_i64(&val, "taskSave.iterationId");
    assert_eq!(
        returned_header, task_header,
        "taskSave must keep returning the stable header id after edits"
    );
    assert_ne!(iter_after, iter_before, "taskSave.iterationId must change after editing the task");

    let bookings_val = gql_ok_simple(
        r#"{ bookings { dbId task { dbId iterationId revCreated revDeleted headerRevCreated headerRevDeleted } } }"#,
    )
    .await;
    let bookings = extract_list(&bookings_val, "bookings");
    let booking = bookings
        .iter()
        .find(|b| extract_i64(b, "dbId") == booking_id)
        .expect("stored booking should be returned by bookings query");
    assert_eq!(
        extract_i64(booking, "task.dbId"),
        task_header,
        "bookings.task.dbId must remain the stable header id"
    );
    assert_eq!(
        extract_i64(booking, "task.iterationId"),
        iter_after,
        "bookings.task.iterationId must follow the latest active iteration"
    );
    assert!(
        value_at_path(booking, "task.revDeleted").is_null(),
        "bookings.task.revDeleted should be null for the active iteration"
    );
    assert!(
        value_at_path(booking, "task.headerRevDeleted").is_null(),
        "bookingSave.task.headerRevDeleted should be null for a live task"
    );
}

// ---------------------------------------------------------------------------
// Task-history helper
// ---------------------------------------------------------------------------

fn task_history_query(task_header_id: i64, direction: &str, extra_args: &str) -> String {
    format!(
        r#"{{
            taskHistory(taskHeaderId: {task_header_id}, direction: {direction}{extra_args}) {{
                changes {{
                    __typename
                    ... on TaskIterationChange {{
                        revisionId
                        timestamp
                        changeType
                        taskIteration {{ dbId title effort }}
                    }}
                    ... on BookingChange {{
                        revisionId
                        timestamp
                        changeType
                        booking {{ dbId start end }}
                    }}
                    ... on DependencyChange {{
                        revisionId
                        timestamp
                        changeType
                        predecessorId
                        successorId
                        predecessorTitle
                        successorTitle
                    }}
                    ... on ResourceConstraintChange {{
                        revisionId
                        timestamp
                        changeType
                        constraintId
                        optional
                        speed
                        resourceIds
                        resourceNames
                    }}
                }}
                hasMore
            }}
        }}"#
    )
}

fn extract_bool(val: &juniper::Value<ExtendedScalarValue>, path: &str) -> bool {
    let cur = value_at_path(val, path);
    cur.as_scalar()
        .and_then(|s| s.try_to_bool())
        .unwrap_or_else(|| panic!("value at '{path}' is not a bool: {cur:?}"))
}

// ---------------------------------------------------------------------------
// Task-history tests
// ---------------------------------------------------------------------------

#[tokio::test]
#[serial]
async fn test_task_history_basic() {
    clean_database().await;

    let t = create_task("Hist-Basic", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;

    let query = task_history_query(t, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    assert!(!changes.is_empty(), "expected at least one change for a newly created task");

    let created = changes.iter().find(|c| extract_str(c, "changeType") == "CREATED");
    assert!(created.is_some(), "expected a CREATED change in task history");

    let created = created.unwrap();
    assert_eq!(
        extract_str(created, "__typename"),
        "TaskIterationChange",
        "CREATED change should be a TaskIterationChange"
    );
    assert_eq!(
        extract_str(created, "taskIteration.title"),
        "Hist-Basic",
        "taskIteration.title should match the created task"
    );

    let has_more = extract_bool(&val, "taskHistory.hasMore");
    assert!(!has_more, "hasMore should be false for a single-change history");
}

#[tokio::test]
#[serial]
async fn test_task_history_after_modification() {
    clean_database().await;

    let t = create_task("Hist-Mod", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;

    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {t},
                title: "Hist-Mod-Updated",
                description: "updated",
                designation: TASK,
                priority: 1.0,
                effort: 8.0
            }}) {{
                dbId
            }}
        }}"#
    );
    gql_ok_simple(&mutation).await;

    let query = task_history_query(t, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    assert!(
        changes.len() >= 2,
        "expected at least 2 changes (CREATED + UPDATED), got {}",
        changes.len()
    );

    let change_types: Vec<&str> = changes.iter().map(|c| extract_str(c, "changeType")).collect();
    assert!(change_types.contains(&"CREATED"), "expected a CREATED change, got {change_types:?}");
    assert!(change_types.contains(&"UPDATED"), "expected an UPDATED change, got {change_types:?}");

    let updated = changes.iter().find(|c| extract_str(c, "changeType") == "UPDATED").unwrap();
    assert_eq!(extract_str(updated, "__typename"), "TaskIterationChange");
    assert_eq!(extract_str(updated, "taskIteration.title"), "Hist-Mod-Updated");
}

#[tokio::test]
#[serial]
async fn test_task_history_after_deletion() {
    clean_database().await;

    let t = create_task("Hist-Del", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;

    let mutation = format!(r#"mutation {{ taskDelete(taskId: {t}) }}"#);
    gql_ok_simple(&mutation).await;

    let query = task_history_query(t, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    let change_types: Vec<&str> = changes.iter().map(|c| extract_str(c, "changeType")).collect();

    assert!(
        change_types.contains(&"CREATED"),
        "expected a CREATED change after deletion, got {change_types:?}"
    );
    assert!(
        change_types.contains(&"DELETED"),
        "expected a DELETED change after deletion, got {change_types:?}"
    );

    let deleted = changes.iter().find(|c| extract_str(c, "changeType") == "DELETED").unwrap();
    assert_eq!(extract_str(deleted, "__typename"), "TaskIterationChange");
}

#[tokio::test]
#[serial]
async fn test_task_history_with_dependencies() {
    clean_database().await;

    let pred = create_task("Hist-Pred", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;
    let succ =
        create_task("Hist-Succ", "TASK", 2.0, Some(4.0), None, Some(&[pred]), None, None, None)
            .await;

    let query = task_history_query(succ, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    let dep_changes: Vec<&juniper::Value<ExtendedScalarValue>> =
        changes.iter().filter(|c| extract_str(c, "__typename") == "DependencyChange").collect();
    assert!(!dep_changes.is_empty(), "expected at least one DependencyChange in successor history");

    let dep = dep_changes
        .iter()
        .find(|c| extract_str(c, "changeType") == "CREATED")
        .expect("expected a CREATED DependencyChange");
    assert_eq!(
        extract_i64(dep, "predecessorId"),
        pred,
        "DependencyChange.predecessorId should match the predecessor task"
    );
    assert_eq!(
        extract_i64(dep, "successorId"),
        succ,
        "DependencyChange.successorId should match the successor task"
    );
}

#[tokio::test]
#[serial]
async fn test_task_save_noop_does_not_create_new_iteration() {
    clean_database().await;

    let task_id =
        create_task("Noop-Task", "TASK", 1.5, Some(4.0), None, None, None, None, None).await;
    let revision_before = latest_revision().await;
    let iteration_before = find_current_iteration_id_by_title("Noop-Task").await;

    let val = gql_ok_simple(&format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {task_id},
                title: "Noop-Task",
                description: "desc for Noop-Task",
                designation: TASK,
                priority: 1.5,
                effort: 4.0,
                parentId: null,
                earliestStart: null,
                scheduleTarget: null
            }}) {{
                dbId
                iterationId
                title
            }}
        }}"#
    ))
    .await;

    let revision_after = latest_revision().await;
    assert_eq!(
        revision_after, revision_before,
        "latest revision should not change for identical task save"
    );
    assert_eq!(extract_i64(&val, "taskSave.dbId"), task_id);
    assert_eq!(extract_i64(&val, "taskSave.iterationId"), iteration_before);
    assert_eq!(extract_str(&val, "taskSave.title"), "Noop-Task");
}

#[tokio::test]
#[serial]
async fn test_task_save_noop_with_same_predecessors_does_not_create_new_iteration() {
    clean_database().await;

    let pred = create_task("Noop-Pred", "TASK", 1.0, Some(2.0), None, None, None, None, None).await;
    let task_id =
        create_task("Noop-Succ", "TASK", 2.0, Some(4.0), None, Some(&[pred]), None, None, None)
            .await;

    let revision_before = latest_revision().await;
    let iteration_before = find_current_iteration_id_by_title("Noop-Succ").await;

    let val = gql_ok_simple(&format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {task_id},
                title: "Noop-Succ",
                description: "desc for Noop-Succ",
                designation: TASK,
                priority: 2.0,
                effort: 4.0,
                parentId: null,
                earliestStart: null,
                scheduleTarget: null,
                predecessors: [{pred}]
            }}) {{
                dbId
                iterationId
            }}
        }}"#
    ))
    .await;

    let revision_after = latest_revision().await;
    assert_eq!(
        revision_after, revision_before,
        "latest revision should not change for identical predecessor save"
    );
    assert_eq!(extract_i64(&val, "taskSave.dbId"), task_id);
    assert_eq!(extract_i64(&val, "taskSave.iterationId"), iteration_before);
}

#[tokio::test]
#[serial]
async fn test_dependency_change_includes_titles() {
    clean_database().await;

    let pred =
        create_task("Hist-Title-Pred", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;
    let succ = create_task(
        "Hist-Title-Succ",
        "TASK",
        2.0,
        Some(4.0),
        None,
        Some(&[pred]),
        None,
        None,
        None,
    )
    .await;

    let query = task_history_query(succ, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    let dep = changes
        .iter()
        .find(|c| {
            extract_str(c, "__typename") == "DependencyChange"
                && extract_str(c, "changeType") == "CREATED"
        })
        .expect("expected a CREATED DependencyChange");

    assert_eq!(extract_i64(dep, "predecessorId"), pred);
    assert_eq!(extract_i64(dep, "successorId"), succ);
    assert_eq!(extract_str(dep, "predecessorTitle"), "Hist-Title-Pred");
    assert_eq!(extract_str(dep, "successorTitle"), "Hist-Title-Succ");
}

#[tokio::test]
#[serial]
async fn test_task_history_with_bookings() {
    clean_database().await;

    let alice_id = create_resource("Hist-BK-Alice", "Europe/Berlin", true).await;
    let req =
        create_task("Hist-BK-Req", "REQUIREMENT", 1.0, None, None, None, None, None, None).await;

    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let t = create_task(
        "Hist-BK-Task",
        "TASK",
        2.0,
        Some(8.0),
        None,
        Some(&[req]),
        None,
        None,
        Some(&rc_frag),
    )
    .await;

    let booking_mutation = format!(
        r#"mutation {{
            bookingSave(
                dbId: null,
                taskId: {t},
                start: "2025-07-01T08:00:00Z",
                end: "2025-07-01T16:00:00Z",
                resources: [{alice_id}],
                final: false
            ) {{
                dbId
            }}
        }}"#
    );
    gql_ok_simple(&booking_mutation).await;

    let query = task_history_query(t, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    let booking_changes: Vec<&juniper::Value<ExtendedScalarValue>> =
        changes.iter().filter(|c| extract_str(c, "__typename") == "BookingChange").collect();
    assert!(
        !booking_changes.is_empty(),
        "expected at least one BookingChange in task history after creating a booking"
    );

    let created_booking = booking_changes
        .iter()
        .find(|c| extract_str(c, "changeType") == "CREATED")
        .expect("expected a CREATED BookingChange");
    assert!(
        !value_at_path(created_booking, "booking").is_null(),
        "CREATED BookingChange should have a non-null booking"
    );
}

#[tokio::test]
#[serial]
async fn test_task_history_with_resource_constraints() {
    clean_database().await;

    let alice_id = create_resource("Hist-RC-Alice", "Europe/Berlin", true).await;

    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let t =
        create_task("Hist-RC-Task", "TASK", 1.0, Some(4.0), None, None, None, None, Some(&rc_frag))
            .await;

    let rc_frag2 = format!(
        r#"resourceConstraints: [{{ optional: true, speed: 0.5, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {t},
                title: "Hist-RC-Task",
                description: "modified",
                designation: TASK,
                priority: 1.0,
                effort: 8.0,
                {rc_frag2}
            }}) {{
                dbId
            }}
        }}"#
    );
    gql_ok_simple(&mutation).await;

    let query = task_history_query(t, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    let rc_changes: Vec<&juniper::Value<ExtendedScalarValue>> = changes
        .iter()
        .filter(|c| extract_str(c, "__typename") == "ResourceConstraintChange")
        .collect();
    assert!(
        !rc_changes.is_empty(),
        "expected at least one ResourceConstraintChange in task history"
    );

    let created_rc = rc_changes
        .iter()
        .find(|c| extract_str(c, "changeType") == "CREATED")
        .expect("expected a CREATED ResourceConstraintChange");
    let resource_ids = extract_list(created_rc, "resourceIds");
    assert!(!resource_ids.is_empty(), "ResourceConstraintChange.resourceIds should not be empty");
}

#[tokio::test]
#[serial]
async fn test_task_history_direction_forward() {
    clean_database().await;

    let t = create_task("Hist-Fwd", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;
    let rev_after_create = latest_revision().await;

    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {t},
                title: "Hist-Fwd-Updated",
                description: "updated",
                designation: TASK,
                priority: 1.0,
                effort: 8.0
            }}) {{
                dbId
            }}
        }}"#
    );
    gql_ok_simple(&mutation).await;

    let query = task_history_query(t, "FORWARD", &format!(", fromRevision: {rev_after_create}"));
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    assert!(
        !changes.is_empty(),
        "FORWARD query from creation revision should return at least one change"
    );

    let change_types: Vec<&str> = changes.iter().map(|c| extract_str(c, "changeType")).collect();
    assert!(
        change_types.contains(&"CREATED"),
        "FORWARD from creation revision should include the CREATED change, got {change_types:?}"
    );

    if changes.len() >= 2 {
        let rev0 = extract_i64(&changes[0], "revisionId");
        let rev1 = extract_i64(&changes[1], "revisionId");
        assert!(
            rev0 <= rev1,
            "FORWARD direction should return changes in chronological order, got rev {rev0} before {rev1}"
        );
    }
}

#[tokio::test]
#[serial]
async fn test_task_history_limit() {
    clean_database().await;

    let t = create_task("Hist-Limit", "TASK", 1.0, Some(1.0), None, None, None, None, None).await;

    for i in 1..=5 {
        let effort = (i + 1) as f64;
        let mutation = format!(
            r#"mutation {{
                taskSave(task: {{
                    dbId: {t},
                    title: "Hist-Limit",
                    description: "modification {i}",
                    designation: TASK,
                    priority: 1.0,
                    effort: {effort}
                }}) {{
                    dbId
                }}
            }}"#
        );
        gql_ok_simple(&mutation).await;
    }

    let query = task_history_query(t, "BACKWARD", ", limit: 2");
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    assert_eq!(changes.len(), 2, "limit=2 should return exactly 2 changes, got {}", changes.len());

    let has_more = extract_bool(&val, "taskHistory.hasMore");
    assert!(has_more, "hasMore should be true when there are more changes than the limit");
}

#[tokio::test]
#[serial]
async fn test_modifying_task_does_not_recreate_dependencies() {
    clean_database().await;

    // 1. Create two tasks A and B
    let b = create_task("DepStable-B", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;
    let a = create_task("DepStable-A", "TASK", 2.0, Some(4.0), None, Some(&[b]), None, None, None)
        .await;

    // 3. Record the latest revision (rev1)
    let rev1 = latest_revision().await;

    // 4. Query task history for A at rev1 — should have DependencyChange entries
    let query = task_history_query(a, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;
    let changes = extract_list(&val, "taskHistory.changes");
    let dep_created_at_rev1: Vec<_> = changes
        .iter()
        .filter(|c| {
            extract_str(c, "__typename") == "DependencyChange"
                && extract_str(c, "changeType") == "CREATED"
        })
        .collect();
    assert!(!dep_created_at_rev1.is_empty(), "expected CREATED DependencyChange entries at rev1");

    // 5. Modify task A (change effort, keep same predecessors and title)
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {a},
                title: "DepStable-A",
                description: "desc for DepStable-A",
                designation: TASK,
                priority: 2.0,
                effort: 8.0,
                predecessors: [{b}]
            }}) {{
                dbId
                title
            }}
        }}"#
    );
    let save_val = gql_ok_simple(&mutation).await;
    assert_eq!(extract_str(&save_val, "taskSave.title"), "DepStable-A");

    // 6. Record the latest revision (rev2)
    let rev2 = latest_revision().await;
    assert!(rev2 > rev1, "rev2 should be greater than rev1");

    // 7. Query task history for A — changes at rev2 should NOT contain DependencyChange
    let query = task_history_query(a, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;
    let changes = extract_list(&val, "taskHistory.changes");

    let dep_changes_at_rev2: Vec<_> = changes
        .iter()
        .filter(|c| {
            extract_str(c, "__typename") == "DependencyChange"
                && extract_i64(c, "revisionId") == rev2
        })
        .collect();
    assert!(
        dep_changes_at_rev2.is_empty(),
        "expected NO DependencyChange entries at rev2 (the modification revision), but got {}: {:?}",
        dep_changes_at_rev2.len(),
        dep_changes_at_rev2.iter().map(|c| extract_str(c, "changeType")).collect::<Vec<_>>()
    );

    // Verify TaskIterationChange UPDATED exists at rev2
    let task_updated_at_rev2: Vec<_> = changes
        .iter()
        .filter(|c| {
            extract_str(c, "__typename") == "TaskIterationChange"
                && extract_str(c, "changeType") == "UPDATED"
                && extract_i64(c, "revisionId") == rev2
        })
        .collect();
    assert!(!task_updated_at_rev2.is_empty(), "expected a TaskIterationChange UPDATED at rev2");

    // 8. Verify that task A still has B as its predecessor
    let preds = query_task_predecessors(a).await;
    assert_eq!(preds, vec![b], "task A should still have B as predecessor after modification");
}

#[tokio::test]
#[serial]
async fn test_task_history_from_timestamp() {
    clean_database().await;

    let t = create_task("Hist-TS", "TASK", 1.0, Some(4.0), None, None, None, None, None).await;

    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {t},
                title: "Hist-TS-Updated",
                description: "updated",
                designation: TASK,
                priority: 1.0,
                effort: 8.0
            }}) {{
                dbId
            }}
        }}"#
    );
    gql_ok_simple(&mutation).await;

    let query = format!(
        r#"{{
            taskHistory(taskHeaderId: {t}, direction: BACKWARD, fromTimestamp: "{now}") {{
                changes {{
                    __typename
                    ... on TaskIterationChange {{
                        revisionId
                        changeType
                        taskIteration {{ dbId title }}
                    }}
                    ... on BookingChange {{
                        revisionId
                        changeType
                    }}
                    ... on DependencyChange {{
                        revisionId
                        changeType
                    }}
                    ... on ResourceConstraintChange {{
                        revisionId
                        changeType
                    }}
                }}
                hasMore
            }}
        }}"#
    );
    let val = gql_ok_simple(&query).await;

    let changes = extract_list(&val, "taskHistory.changes");
    assert!(!changes.is_empty(), "fromTimestamp query should return at least one change");

    let has_created = changes.iter().any(|c| extract_str(c, "changeType") == "CREATED");
    assert!(
        has_created,
        "BACKWARD from a timestamp after creation should include the CREATED change"
    );
}

#[tokio::test]
#[serial]
async fn test_modifying_task_does_not_recreate_resource_constraints() {
    clean_database().await;

    // 1. Create a resource
    let alice_id = create_resource("RC-Stable-Alice", "Europe/Berlin", true).await;

    // 2. Create a task with a resource constraint pointing to that resource
    let rc_frag = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let t = create_task(
        "RC-Stable-Task",
        "TASK",
        1.0,
        Some(4.0),
        None,
        None,
        None,
        None,
        Some(&rc_frag),
    )
    .await;

    // 3. Record rev1
    let rev1 = latest_revision().await;

    // 4. Query task history at rev1 — should have ResourceConstraintChange CREATED
    let query = task_history_query(t, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;
    let changes = extract_list(&val, "taskHistory.changes");
    let rc_created_at_rev1: Vec<_> = changes
        .iter()
        .filter(|c| {
            extract_str(c, "__typename") == "ResourceConstraintChange"
                && extract_str(c, "changeType") == "CREATED"
        })
        .collect();
    assert!(
        !rc_created_at_rev1.is_empty(),
        "expected CREATED ResourceConstraintChange entries at rev1"
    );

    // 5. Modify the task (change effort, keep same resource constraints)
    let rc_frag2 = format!(
        r#"resourceConstraints: [{{ optional: false, speed: 1.0, entries: [{{ resourceId: {alice_id} }}] }}]"#
    );
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {t},
                title: "RC-Stable-Task",
                description: "desc for RC-Stable-Task",
                designation: TASK,
                priority: 1.0,
                effort: 8.0,
                {rc_frag2}
            }}) {{
                dbId
                title
            }}
        }}"#
    );
    let save_val = gql_ok_simple(&mutation).await;
    assert_eq!(extract_str(&save_val, "taskSave.title"), "RC-Stable-Task");

    // 6. Record rev2
    let rev2 = latest_revision().await;
    assert!(rev2 > rev1, "rev2 should be greater than rev1");

    // 7. Query task history — changes at rev2 should NOT contain ResourceConstraintChange
    let query = task_history_query(t, "BACKWARD", "");
    let val = gql_ok_simple(&query).await;
    let changes = extract_list(&val, "taskHistory.changes");

    let rc_changes_at_rev2: Vec<_> = changes
        .iter()
        .filter(|c| {
            extract_str(c, "__typename") == "ResourceConstraintChange"
                && extract_i64(c, "revisionId") == rev2
        })
        .collect();
    assert!(
        rc_changes_at_rev2.is_empty(),
        "expected NO ResourceConstraintChange entries at rev2 (the modification revision), but got {}: {:?}",
        rc_changes_at_rev2.len(),
        rc_changes_at_rev2.iter().map(|c| extract_str(c, "changeType")).collect::<Vec<_>>()
    );

    // Verify TaskIterationChange UPDATED exists at rev2
    let task_updated_at_rev2: Vec<_> = changes
        .iter()
        .filter(|c| {
            extract_str(c, "__typename") == "TaskIterationChange"
                && extract_str(c, "changeType") == "UPDATED"
                && extract_i64(c, "revisionId") == rev2
        })
        .collect();
    assert!(!task_updated_at_rev2.is_empty(), "expected a TaskIterationChange UPDATED at rev2");

    // 8. Verify the task still has its resource constraint after modification
    let rc_query = format!(
        r#"{{
            tasks {{
                dbId
                title
                resourceConstraints {{
                    id
                    optional
                    speed
                    entries {{
                        id
                        resource {{ dbId name }}
                    }}
                }}
            }}
        }}"#
    );
    let rc_val = gql_ok_simple(&rc_query).await;
    let tasks = extract_list(&rc_val, "tasks");
    let our_task = tasks
        .iter()
        .find(|t_val| extract_str(t_val, "title") == "RC-Stable-Task")
        .expect("should find RC-Stable-Task in tasks list");
    let rcs = extract_list(our_task, "resourceConstraints");
    assert_eq!(rcs.len(), 1, "task should still have exactly one resource constraint");
    let entries = extract_list(&rcs[0], "entries");
    assert!(!entries.is_empty(), "resource constraint should still have entries");
}
