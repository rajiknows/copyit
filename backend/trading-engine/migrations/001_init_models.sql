
CREATE TABLE traders (
    address TEXT PRIMARY KEY,
    name TEXT,
    added_at TIMESTAMP DEFAULT NOW(),
    is_active BOOLEAN DEFAULT true
);

CREATE TABLE followers (
    id SERIAL PRIMARY KEY,
    address TEXT UNIQUE NOT NULL,
    agent_signature TEXT NOT NULL,  -- stored once, signed by user
    created_at TIMESTAMP DEFAULT NOW()
);

-- Copy config: which trader each follower copies + ratio
CREATE TABLE copy_configs (
    id SERIAL PRIMARY KEY,
    follower_id INT REFERENCES followers(id) ON DELETE CASCADE,
    trader_address TEXT REFERENCES traders(address) ON DELETE CASCADE,
    ratio DECIMAL(10,6) NOT NULL DEFAULT 0.01,  -- 1% by default
    is_active BOOLEAN DEFAULT true,
    max_risk_per_trade DECIMAL(20,8),  -- optional: $500 max per trade
    UNIQUE(follower_id, trader_address)
);

-- Executed trades (for PnL tracking, leaderboard, audit)
CREATE TABLE executed_trades (
    id BIGSERIAL PRIMARY KEY,
    follower_address TEXT NOT NULL,
    trader_address TEXT NOT NULL,
    coin TEXT NOT NULL,
    side TEXT NOT NULL,        -- "B" or "A"
    size DECIMAL(20,8) NOT NULL,
    price DECIMAL(20,8) NOT NULL,
    order_hash TEXT,
    hl_oid BIGINT,
    timestamp TIMESTAMP DEFAULT NOW(),
    status TEXT DEFAULT 'sent'  -- sent, filled, failed
);

-- Leaderboard cache (updated daily)
CREATE TABLE leaderboard (
    trader_address TEXT PRIMARY KEY,
    pnl_percent_30d DECIMAL(10,4),
    win_rate DECIMAL(5,2),
    sharpe DECIMAL(6,3),
    max_drawdown DECIMAL(10,4),
    followers_count INT DEFAULT 0,
    volume_7d DECIMAL(20,8),
    updated_at TIMESTAMP DEFAULT NOW()
);
