-- Add migration script here
CREATE TABLE Users
(
    id            VARCHAR(36) PRIMARY KEY,
    display_name  VARCHAR(50) NOT NULL CHECK (LENGTH(display_name) < 50),
    role          VARCHAR(20) NOT NULL CHECK (role IN ('Admin', 'Regular')),
    team          VARCHAR(36) DEFAULT NULL REFERENCES Teams (id) ON DELETE SET NULL ON UPDATE CASCADE,
    refresh_token VARCHAR(50) DEFAULT NULL
);

CREATE TABLE Teams
(
    id           VARCHAR(36) PRIMARY KEY,
    display_name VARCHAR(255) NOT NULL CHECK (LENGTH(display_name) < 255)
);

CREATE TABLE TeamIntegrationTokens
(
    team_id VARCHAR(36) PRIMARY KEY REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    token   VARCHAR(40) NOT NULL
);

CREATE TABLE Repos
(
    team VARCHAR(36) PRIMARY KEY REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    url  VARCHAR(255) NOT NULL
);

CREATE TABLE Queue
(
    id             VARCHAR(36) PRIMARY KEY,
    team           VARCHAR(36)  NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    revision       VARCHAR(255) NOT NULL,
    commit_message VARCHAR(120) NOT NULL,
    insert_time    INTEGER      NOT NULL
);

CREATE TABLE Tasks
(
    task_id        VARCHAR(36) PRIMARY KEY,
    team_id        VARCHAR(36)  NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    revision       VARCHAR(40)  NOT NULL,
    commit_message VARCHAR(120) NOT NULL,
    queue_time     INTEGER      NOT NULL,
    start_time     INTEGER      NOT NULL,
    end_time       INTEGER      NOT NULL,
    execution_id   VARCHAR(36) DEFAULT NULL REFERENCES ExecutionResults (execution_id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE TestResults
(
    task_id          VARCHAR(36) NOT NULL REFERENCES Tasks (task_id) ON DELETE CASCADE ON UPDATE CASCADE,
    test_id          VARCHAR(36) NOT NULL REFERENCES Tests (id) ON DELETE CASCADE ON UPDATE CASCADE,
    compiler_exec_id VARCHAR(36) NOT NULL REFERENCES ExecutionResults (execution_id) ON DELETE CASCADE ON UPDATE CASCADE,
    binary_exec_id   VARCHAR(36) REFERENCES ExecutionResults (execution_id) ON DELETE CASCADE ON UPDATE CASCADE,
    status           VARCHAR(40) NOT NULL,

    PRIMARY KEY (task_id, test_id)
);

CREATE TABLE ExecutionResults
(
    execution_id VARCHAR(36) PRIMARY KEY,
    stdout       TEXT        NOT NULL,
    stderr       TEXT        NOT NULL,
    error        TEXT,
    result       VARCHAR(30) NOT NULL,
    duration_ms  INTEGER     NOT NULL,
    exit_code    INTEGER
);

CREATE TABLE Tests
(
    id                 VARCHAR(120) PRIMARY KEY,
    owner              VARCHAR(36) NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    compiler_modifiers TEXT        NOT NULL, -- json serialized modifiers
    binary_modifiers   TEXT        NOT NULL, -- json serialized modifiers
    admin_authored     BOOLEAN     NOT NULL,
    category           VARCHAR(10) NOT NULL,
    hash               VARCHAR(40) NOT NULL,
    provisional        BOOLEAN     NOT NULL, -- if true, the test was submitted after the test deadline
    last_updated       INTEGER     NOT NULL DEFAULT (CAST(unixepoch('subsec') * 1000 as INTEGER))
);

CREATE TABLE TestTastingResults
(
    test_id          VARCHAR(120) NOT NULL REFERENCES Tests (id) ON DELETE CASCADE ON UPDATE CASCADE,
    compiler_exec_id VARCHAR(36)  NOT NULL REFERENCES ExecutionResults (execution_id) ON DELETE CASCADE ON UPDATE CASCADE,
    binary_exec_id   VARCHAR(36) REFERENCES ExecutionResults (execution_id) ON DELETE CASCADE ON UPDATE CASCADE,
    status           VARCHAR(40)  NOT NULL,

    PRIMARY KEY (test_id)
);

CREATE TABLE ExternalRuns
(
    task_id  VARCHAR(36)  NOT NULL,
    run_id   INTEGER      NOT NULL,
    platform VARCHAR(30)  NOT NULL,
    repo     VARCHAR(255) NOT NULL,
    owner    VARCHAR(255) NOT NULL,
    revision VARCHAR(40)  NOT NULL,
    status   VARCHAR(30)  NOT NULL,

    PRIMARY KEY (run_id, platform)
);

CREATE TABLE ManuallySubmittedTasks
(
    team_id     VARCHAR(36) NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    category    VARCHAR(10) NOT NULL,
    task_id     VARCHAR(36) REFERENCES Tasks (task_id) ON DELETE CASCADE ON UPDATE CASCADE,
    -- user kept for disputes...
    user_id     VARCHAR(36) NOT NULL REFERENCES Users (id) ON DELETE CASCADE ON UPDATE CASCADE,
    update_time INTEGER     NOT NULL,

    PRIMARY KEY (team_id, category)
);