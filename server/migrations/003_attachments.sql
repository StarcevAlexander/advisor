CREATE TABLE attachments (
    id BIGSERIAL PRIMARY KEY,
    conversation_id TEXT NOT NULL,
    turn_id TEXT NOT NULL,
    name TEXT NOT NULL,
    data BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX attachments_turn_idx ON attachments (conversation_id, turn_id);
