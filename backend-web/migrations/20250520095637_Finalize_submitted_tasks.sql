CREATE TABLE FinalizedSubmittedTasks
(
    team_id  VARCHAR(36) NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    task_id  VARCHAR(36) NOT NULL,
    category VARCHAR(10) NOT NULL,

    PRIMARY KEY (team_id, category)
);