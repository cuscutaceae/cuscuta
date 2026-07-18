CREATE TABLE IF NOT EXISTS account_table (
  id BIGSERIAL PRIMARY KEY,
  account_email TEXT NOT NULL,
  account_password TEXT NOT NULL,
  user_id INTEGER DEFAULT NULL,
  temp_token TEXT DEFAULT NULL,
  state TEXT NOT NULL DEFAULT 'Idle',
  rate INTEGER NOT NULL DEFAULT 10,
  lease_time TIMESTAMPTZ NOT NULL DEFAULT now()
);