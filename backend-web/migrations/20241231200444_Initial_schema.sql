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

CREATE TABLE Repos
(
    team       VARCHAR(36)  NOT NULL REFERENCES Teams (id) ON DELETE CASCADE ON UPDATE CASCADE,
    url        VARCHAR(255) NOT NULL,
    auto_fetch BOOLEAN      NOT NULL DEFAULT TRUE
);