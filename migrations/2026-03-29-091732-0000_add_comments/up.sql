CREATE TABLE comments (
    id INTEGER PRIMARY KEY NOT NULL,
    project_id INTEGER NOT NULL
        REFERENCES projects ON DELETE CASCADE,
    date DATE NOT NULL,
    duration_minutes INTEGER,
    text TEXT NOT NULL
);
