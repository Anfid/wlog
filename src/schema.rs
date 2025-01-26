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
    schedule_logs (project_id, month) {
        project_id -> Integer,
        month -> Integer,
        bitmap -> Integer,
    }
}

diesel::table! {
    schedule_settings (project_id) {
        project_id -> Integer,
        weekdays -> Nullable<Integer>,
        workday_minutes -> Nullable<Integer>,
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
diesel::joinable!(schedule_logs -> projects (project_id));
diesel::joinable!(schedule_settings -> projects (project_id));
diesel::joinable!(tasks -> projects (project_id));

diesel::allow_tables_to_appear_in_same_query!(
    default_project,
    log_entries,
    projects,
    schedule_logs,
    schedule_settings,
    tasks,
);
