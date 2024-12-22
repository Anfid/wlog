CREATE TABLE log_entries (
    date DATE NOT NULL,
    task_id INTEGER NOT NULL
        REFERENCES tasks ON DELETE CASCADE,
    duration_minutes INTEGER NOT NULL,
    PRIMARY KEY (date, task_id)
);

CREATE TABLE tasks (
    id INTEGER PRIMARY KEY NOT NULL,
    project_id INTEGER NOT NULL
        REFERENCES projects ON DELETE CASCADE,
    name TEXT NOT NULL,
    issue INTEGER,
    description TEXT
);

CREATE TABLE projects (
    id INTEGER PRIMARY KEY NOT NULL,
    url TEXT NOT NULL,
    name TEXT UNIQUE
);

CREATE TABLE default_project (
    id INTEGER PRIMARY KEY NOT NULL CHECK (id = 0),
    project_id INTEGER NOT NULL
        REFERENCES projects ON DELETE CASCADE
);
