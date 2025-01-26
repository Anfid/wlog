CREATE TABLE schedule_logs (
    project_id INTEGER NOT NULL
        REFERENCES projects ON DELETE CASCADE,
    month INTEGER NOT NULL,
    bitmap INTEGER NOT NULL,
    PRIMARY KEY (project_id, month)
);

CREATE TABLE schedule_settings (
    project_id INTEGER PRIMARY KEY NOT NULL
        REFERENCES projects ON DELETE CASCADE,
    weekdays INTEGER,
    workday_minutes INTEGER
);
