//! Integration tests for the SIAPLA GraphQL API against a real SQLite database.
//!
//! All tests share a single temporary database (because `GLOBAL_DATABASE_URL` uses a
//! process-wide `OnceCell`). Tests are run serially via `#[serial]` to avoid
//! concurrency issues on the shared database.

use std::sync::Arc;

use tokio::sync::OnceCell;

use juniper::{ScalarValue as _, Variables};
use sea_orm::{Database, DatabaseConnection, TransactionTrait as _};
use serial_test::serial;
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
        "task",
        "resource",
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

    txn.commit().await.expect("clean_database: commit failed");
    db.close().await.ok();
}

/// Execute a GraphQL **query or mutation** and return the result value.
/// Panics on transport/execution errors.
async fn gql_exec(
    query: &str,
    vars: &Variables,
) -> (juniper::Value, Vec<juniper::ExecutionError<juniper::DefaultScalarValue>>) {
    let schema = schema();
    let (app_state, _manual_rx) = AppState::new();
    let ctx = Context::new(app_state);
    let (val, errors) = juniper::execute(query, None, &schema, vars, &*ctx)
        .await
        .expect("GraphQL execution failed");

    // Commit the transaction so data is persisted for subsequent operations.
    let mut ctx_owned =
        Arc::into_inner(ctx).expect("Context Arc should have no other strong references");
    ctx_owned.commit().await.expect("commit failed");

    (val, errors)
}

/// Shorthand: execute, assert zero errors, return the data `Value`.
async fn gql_ok(query: &str, vars: &Variables) -> juniper::Value {
    let (val, errors) = gql_exec(query, vars).await;
    assert!(errors.is_empty(), "Expected no GraphQL errors but got: {errors:?}\nQuery: {query}");
    val
}

/// Shorthand: execute with empty variables.
async fn gql_ok_simple(query: &str) -> juniper::Value {
    gql_ok(query, &Variables::new()).await
}

/// Get the juniper value at the given dot path
fn value_at_path<'a>(val: &'a juniper::Value, path: &str) -> &'a juniper::Value {
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
fn extract_i64(val: &juniper::Value, path: &str) -> i64 {
    let cur = value_at_path(val, path);
    let s = cur.as_scalar().unwrap_or_else(|| panic!("value at '{path}' is not a scalar: {cur:?}"));
    s.try_to_int()
        .map(|v| v as i64)
        .unwrap_or_else(|| panic!("value at '{path}' is not an int: {cur:?}"))
}

fn extract_str<'a>(val: &'a juniper::Value, path: &str) -> &'a str {
    let cur = value_at_path(val, path);
    cur.as_scalar()
        .and_then(|s| s.try_as_str())
        .unwrap_or_else(|| panic!("value at '{path}' is not a string: {cur:?}"))
}

fn extract_list<'a>(val: &'a juniper::Value, path: &str) -> &'a Vec<juniper::Value> {
    let cur = value_at_path(val, path);
    cur.as_list_value().unwrap_or_else(|| panic!("value at '{path}' is not a list: {cur:?}"))
}

fn extract_f64(val: &juniper::Value, path: &str) -> f64 {
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

/// Query predecessors for a task by its dbId using a fresh GraphQL context.
/// Returns a sorted list of predecessor dbIds.
async fn query_task_predecessors(task_id: i64) -> Vec<i64> {
    let val = gql_ok_simple(r#"{ tasks { dbId predecessors { dbId } } }"#).await;
    let tasks = extract_list(&val, "tasks");
    let task = tasks
        .iter()
        .find(|t| extract_i64(t, "dbId") == task_id)
        .unwrap_or_else(|| panic!("task with dbId={task_id} not found"));
    let preds = extract_list(task, "predecessors");
    let mut ids: Vec<i64> = preds.iter().map(|p| extract_i64(p, "dbId")).collect();
    ids.sort();
    ids
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
        let preds_str = format!("{:?}", preds);
        let mutation = format!(
            r#"mutation {{
                taskSave(task: {{
                    dbId: {id},
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
        async move {
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
    assert_eq!(preds, vec![task_a], "Task->Task predecessor mismatch");

    // 4. Task → Group (change group's predecessor from req to task_a)
    update_task_preds(group, "Dep-Group", "GROUP", &[task_a]).await;
    let preds = query_task_predecessors(group).await;
    assert_eq!(preds, vec![task_a], "Task->Group predecessor mismatch");

    // 5. Group → Task (change task_b's predecessor from task_a to group)
    update_task_preds(task_b, "Dep-TaskB", "TASK", &[group]).await;
    let preds = query_task_predecessors(task_b).await;
    assert_eq!(preds, vec![group], "Group->Task predecessor mismatch");

    // 6. Task → Milestone
    update_task_preds(ms, "Dep-Milestone", "MILESTONE", &[task_b]).await;
    let preds = query_task_predecessors(ms).await;
    assert_eq!(preds, vec![task_b], "Task->Milestone predecessor mismatch");

    // 7. Group → Milestone (change milestone's predecessor from task_b to group)
    update_task_preds(ms, "Dep-Milestone", "MILESTONE", &[group]).await;
    let preds = query_task_predecessors(ms).await;
    assert_eq!(preds, vec![group], "Group->Milestone predecessor mismatch");
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
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {group_a},
                title: "GroupA",
                description: "top group",
                designation: GROUP,
                priority: 1.0,
                children: [{group_b}]
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
    let child_id = extract_i64(&children_a[0], "dbId");
    assert_eq!(child_id, group_b, "expected GroupA child to be GroupB ({group_b}), got {child_id}");

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
    let mutation = format!(
        r#"mutation {{
            taskSave(task: {{
                dbId: {t2},
                title: "Chain-T2",
                description: "updated",
                designation: TASK,
                priority: 3.0,
                effort: 4.0,
                predecessors: [{req}]
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
