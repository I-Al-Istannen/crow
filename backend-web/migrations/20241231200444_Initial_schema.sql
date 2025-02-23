-- Add migration script here
CREATE TABLE Users
(
    id           VARCHAR(36) PRIMARY KEY,
    display_name VARCHAR(50) NOT NULL CHECK (LENGTH(display_name) < 50),
    role         VARCHAR(20) NOT NULL CHECK (role IN ('Admin', 'Regular')),
    team         VARCHAR(36) DEFAULT NULL REFERENCES Teams (id) ON DELETE SET NULL ON UPDATE CASCADE
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
    team VARCHAR(36)  NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
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
    start_time     INTEGER      NOT NULL,
    end_time       INTEGER      NOT NULL,
    execution_id   VARCHAR(36) DEFAULT NULL REFERENCES ExecutionResults (execution_id) ON DELETE CASCADE ON UPDATE CASCADE
);

CREATE TABLE TestResults
(
    task_id      VARCHAR(36) NOT NULL REFERENCES Tasks (task_id) ON DELETE CASCADE ON UPDATE CASCADE,
    test_id      VARCHAR(36) NOT NULL,
    execution_id VARCHAR(36) NOT NULL REFERENCES ExecutionResults (execution_id) ON DELETE CASCADE ON UPDATE CASCADE,

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
    id              VARCHAR(120) PRIMARY KEY,
    expected_output TEXT        NOT NULL,
    input           TEXT        NOT NULL,
    owner           VARCHAR(36) NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    admin_authored  BOOLEAN     NOT NULL,
    category        VARCHAR(10) NOT NULL
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