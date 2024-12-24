// @generated automatically by Diesel CLI.

diesel::table! {
    default_project (id) {
        id -> Integer,
        project_id -> Integer,
    }
}

diesel::table! {
    log_entries (date, task_id) {
        date -> Date,
        task_id -> Integer,
        duration_minutes -> Integer,
    }
}

diesel::table! {
    projects (id) {
        id -> Integer,
        url -> Text,
        name -> Nullable<Text>,
    }
}

diesel::table! {
    tasks (id) {
        id -> Integer,
        project_id -> Integer,
        name -> Text,
        issue -> Nullable<Integer>,
    }
}

diesel::joinable!(default_project -> projects (project_id));
diesel::joinable!(log_entries -> tasks (task_id));
diesel::joinable!(tasks -> projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    default_project,
    log_entries,
    projects,
    tasks,
);
