CREATE TABLE ogn_positions (
  ogn_id TEXT NOT NULL,
  time TIMESTAMP NOT NULL,
  longitude FLOAT NOT NULL,
  latitude FLOAT NOT NULL,
  PRIMARY KEY (time, ogn_id)
);

-- see https://www.timescale.com/
SELECT create_hypertable('ogn_positions', 'time', chunk_time_interval => interval '5 minutes');
