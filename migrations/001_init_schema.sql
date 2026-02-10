-- Notif: users, domains (1 domain = 1 API key), channels, ws_connections
-- Run with: psql $DATABASE_URL -f migrations/001_init_schema.sql

CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Users
CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX idx_users_email ON users(email);

-- Domains: 1 domain = 1 API key (domain_name + key per user)
CREATE TABLE domains (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    domain_name VARCHAR(255) NOT NULL,
    key VARCHAR(64) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_active BOOLEAN NOT NULL DEFAULT true,
    UNIQUE(user_id, domain_name)
);
CREATE INDEX idx_domains_user_id ON domains(user_id);
CREATE INDEX idx_domains_key ON domains(key);

-- Channels (per domain for monitoring)
CREATE TABLE channels (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    name VARCHAR(255) NOT NULL,
    domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(name, domain_id)
);
CREATE INDEX idx_channels_domain_id ON channels(domain_id);
CREATE INDEX idx_channels_name ON channels(name);

-- WebSocket connections (monitoring)
CREATE TABLE ws_connections (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    channel_id UUID REFERENCES channels(id) ON DELETE SET NULL,
    channel_name VARCHAR(255) NOT NULL,
    domain_id UUID NOT NULL REFERENCES domains(id) ON DELETE CASCADE,
    socket_id VARCHAR(128) NOT NULL,
    connected_user VARCHAR(255),
    connected_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    disconnected_at TIMESTAMPTZ,
    status VARCHAR(32) NOT NULL DEFAULT 'connected' CHECK (status IN ('connected', 'disconnected'))
);
CREATE INDEX idx_ws_connections_channel ON ws_connections(channel_name);
CREATE INDEX idx_ws_connections_status ON ws_connections(status);
CREATE INDEX idx_ws_connections_domain ON ws_connections(domain_id);

-- RLS
ALTER TABLE users ENABLE ROW LEVEL SECURITY;
ALTER TABLE domains ENABLE ROW LEVEL SECURITY;
ALTER TABLE channels ENABLE ROW LEVEL SECURITY;
ALTER TABLE ws_connections ENABLE ROW LEVEL SECURITY;
CREATE POLICY users_own ON users FOR ALL USING (true);
CREATE POLICY domains_own ON domains FOR ALL USING (true);
CREATE POLICY channels_own ON channels FOR ALL USING (true);
CREATE POLICY ws_connections_own ON ws_connections FOR ALL USING (true);

COMMENT ON TABLE domains IS '1 domain = 1 API key; domain_name is the allowed origin';
COMMENT ON TABLE channels IS 'Channel names per domain for monitoring';
COMMENT ON TABLE ws_connections IS 'Active WebSocket connections for monitoring';
