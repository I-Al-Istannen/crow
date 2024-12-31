-- Add migration script here
CREATE TABLE Users
(
    id           VARCHAR(36) PRIMARY KEY,
    display_name VARCHAR(255) NOT NULL,
    role         VARCHAR(20)  NOT NULL,
    team         VARCHAR(36) DEFAULT NULL REFERENCES Teams (id) ON DELETE SET NULL ON UPDATE CASCADE
);

CREATE TABLE Teams
(
    id           VARCHAR(36) PRIMARY KEY,
    display_name VARCHAR(255) NOT NULL
);