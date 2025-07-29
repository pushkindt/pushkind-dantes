-- Your SQL goes here
CREATE TABLE crawlers (
    id INTEGER NOT NULL PRIMARY KEY,
    hub_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    selector TEXT NOT NULL,
    processing BOOLEAN NOT NULL DEFAULT FALSE,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

INSERT INTO crawlers (id, hub_id, name, url, selector)
VALUES (1, 1, '101tea', 'https://101tea.ru', '101tea');
