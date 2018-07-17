CREATE TABLE ogn_devices (
  ogn_id TEXT NOT NULL PRIMARY KEY,
  model TEXT,
  category SMALLINT NOT NULL,
  registration TEXT,
  callsign TEXT
);
