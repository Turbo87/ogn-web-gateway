-- Firstly, remove PRIMARY KEY attribute of former PRIMARY KEY
ALTER TABLE ogn_positions DROP CONSTRAINT ogn_positions_pkey;

-- Lastly set your new PRIMARY KEY
ALTER TABLE ogn_positions ADD PRIMARY KEY (time, ogn_id);
