use sqlx::PgPool;
use rust_decimal::{Decimal, MathematicalOps};
use rust_decimal_macros::dec;
use std::collections::HashMap;

#[derive(Debug, sqlx::FromRow)]
struct TradeRecord {
    trader_address: String,
    closed_pnl: Option<Decimal>,
    entry_px: Decimal,
    exit_px: Decimal,
    sz: Decimal,
    is_long: bool,
    timestamp: chrono::DateTime<chrono::Utc>,
}

pub async fn update_all_leaderboards(pool: &PgPool) -> anyhow::Result<()> {
    let traders = sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT trader_address FROM executed_trades WHERE timestamp > NOW() - INTERVAL '90 days'"
    )
    .fetch_all(pool)
    .await?;

    for trader in traders {
        let metrics = calculate_trader_metrics(pool, &trader).await?;

        sqlx::query(
            r#"
            INSERT INTO leaderboard (
                trader_address, pnl_percent_30d, win_rate, sharpe, max_drawdown,
                followers_count, volume_7d, updated_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, NOW())
            ON CONFLICT (trader_address) DO UPDATE SET
                pnl_percent_30d = EXCLUDED.pnl_percent_30d,
                win_rate = EXCLUDED.win_rate,
                sharpe = EXCLUDED.sharpe,
                max_drawdown = EXCLUDED.max_drawdown,
                followers_count = EXCLUDED.followers_count,
                volume_7d = EXCLUDED.volume_7d,
                updated_at = NOW()
            "#
        )
        .bind(&trader)
        .bind(metrics.pnl_percent_30d)
        .bind(metrics.win_rate)
        .bind(metrics.sharpe)
        .bind(metrics.max_drawdown)
        .bind(metrics.followers_count as i32)
        .bind(metrics.volume_7d)
        .execute(pool)
        .await?;
    }

    Ok(())
}

struct TraderMetrics {
    pnl_percent_30d: Decimal,
    win_rate: Decimal,
    sharpe: Decimal,
    max_drawdown: Decimal,
    followers_count: i64,
    volume_7d: Decimal,
}

async fn calculate_trader_metrics(pool: &PgPool, trader: &str) -> anyhow::Result<TraderMetrics> {
    // 30-day PnL %
    let pnl_30d: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(closed_pnl), 0) FROM executed_trades
         WHERE trader_address = $1 AND timestamp > NOW() - INTERVAL '30 days'"
    )
    .bind(trader)
    .fetch_one(pool)
    .await?;

    // Approximate initial capital from average position size (or use known deposit data)
    let avg_position: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(AVG(sz * entry_px), 100000) FROM executed_trades
         WHERE trader_address = $1 AND timestamp > NOW() - INTERVAL '90 days'"
    )
    .bind(trader)
    .fetch_one(pool)
    .await?;

    let pnl_percent_30d = if avg_position > dec!(0) {
        (pnl_30d / avg_position) * dec!(100)
    } else {
        dec!(0)
    };

    // Win rate
    let wins: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM executed_trades
         WHERE trader_address = $1 AND closed_pnl > 0 AND timestamp > NOW() - INTERVAL '30 days'"
    )
    .bind(trader)
    .fetch_one(pool)
    .await?;

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM executed_trades
         WHERE trader_address = $1 AND closed_pnl IS NOT NULL AND timestamp > NOW() - INTERVAL '30 days'"
    )
    .bind(trader)
    .fetch_one(pool)
    .await?;

    let win_rate = if total > 0 {
        Decimal::from(wins) / Decimal::from(total) * dec!(100)
    } else {
        dec!(0)
    };

    // Simplified Sharpe (daily returns std dev approximation)
    let daily_returns: Vec<Decimal> = sqlx::query_scalar(
        "SELECT SUM(closed_pnl) FROM executed_trades
         WHERE trader_address = $1
         GROUP BY DATE(timestamp)
         ORDER BY DATE(timestamp) DESC
         LIMIT 30"
    )
    .fetch_all(pool)
    .await?;

    let mean: Decimal = daily_returns.iter().sum::<Decimal>() / Decimal::from(daily_returns.len().max(1));
    let variance: Decimal = daily_returns.iter()
        .map(|r| (*r - mean).powi(2))
        .sum::<Decimal>() / Decimal::from(daily_returns.len().max(1));
    let sharpe = if variance > dec!(0) {
        if let Some(variance_sqrt) = variance.sqrt(){
            mean / variance_sqrt
        }else{
            dec!(0)
        }
    }else{
        dec!(0)
    };


    // Max drawdown (simplified from equity curve)
    let mut equity_curve = vec![dec!(0)];
    let mut current = dec!(0);
    let pnls: Vec<Decimal> = sqlx::query_scalar(
        "SELECT closed_pnl FROM executed_trades
         WHERE trader_address = $1 AND timestamp > NOW() - INTERVAL '90 days'
         ORDER BY timestamp"
    )
    .fetch_all(pool)
    .await?;

    let mut peak = dec!(0);
    let mut max_dd = dec!(0);
    for pnl in pnls {
        current += pnl;
        equity_curve.push(current);
        if current > peak {
            peak = current;
        }
        let dd = (peak - current) / peak.max(dec!(1));
        if dd > max_dd {
            max_dd = dd;
        }
    }

    // Followers count
    let followers_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT follower_id) FROM copy_configs
         WHERE trader_address = $1 AND is_active = true"
    )
    .bind(trader)
    .fetch_one(pool)
    .await?;

    // 7-day volume
    let volume_7d: Decimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(sz * exit_px), 0) FROM executed_trades
         WHERE trader_address = $1 AND timestamp > NOW() - INTERVAL '7 days'"
    )
    .bind(trader)
    .fetch_one(pool)
    .await?;

    Ok(TraderMetrics {
        pnl_percent_30d: pnl_percent_30d.round_dp(2),
        win_rate: win_rate.round_dp(2),
        sharpe: sharpe.round_dp(3),
        max_drawdown: (max_dd * dec!(100)).round_dp(2),
        followers_count,
        volume_7d,
    })
}



#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::SqlitePoolOptions;
    use chrono::{Utc, Duration};
    use rust_decimal_macros::dec;

    async fn setup_test_db() -> sqlx::Pool<sqlx::Sqlite> {
        let pool = SqlitePoolOptions::new()
            .connect("sqlite::memory:")
            .await
            .unwrap();

        // Create tables (SQLite-compatible schema)
        pool.execute(
            r#"
            CREATE TABLE executed_trades (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            trader_address TEXT NOT NULL,
            closed_pnl TEXT,  -- SQLite has no DECIMAL; store as TEXT and parse
            entry_px TEXT NOT NULL,
            exit_px TEXT NOT NULL,
            sz TEXT NOT NULL,
            is_long INTEGER NOT NULL,
            timestamp TEXT NOT NULL
            );
            CREATE TABLE copy_configs (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            follower_id INTEGER,
            trader_address TEXT NOT NULL,
            is_active INTEGER DEFAULT 1
            );
            CREATE TABLE followers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            address TEXT NOT NULL
            );
            CREATE TABLE leaderboard (
            trader_address TEXT PRIMARY KEY,
            pnl_percent_30d TEXT,
            win_rate TEXT,
            sharpe TEXT,
            max_drawdown TEXT,
            followers_count INTEGER,
            volume_7d TEXT,
            updated_at TEXT
            );
            "#
        ).await.unwrap();

        pool
    }

    #[tokio::test]
    async fn test_leaderboard_basic_pnl_calculation() {
        let pool = setup_test_db().await;
        let trader = "0x123";

        // Insert profitable trades
        sqlx::query(
            r#"
            INSERT INTO executed_trades
            (trader_address, closed_pnl, entry_px, exit_px, sz, is_long, timestamp)
            VALUES
            (?, 1000.0, 50000.0, 51000.0, 1.0, 1, ?),
            (?, 500.0, 49000.0, 49500.0, 2.0, 1, ?)
            "#
        )
        .bind(trader)
        .bind(Utc::now())
        .bind(trader)
        .bind(Utc::now() - Duration::days(10))
        .execute(&pool)
        .await
        .unwrap();

        let metrics = calculate_trader_metrics(&pool, trader).await.unwrap();

        assert!(metrics.pnl_percent_30d > dec!(0));
        assert!(metrics.win_rate == dec!(100.0)); // both trades profitable
        assert!(metrics.sharpe > dec!(0));
        assert!(metrics.max_drawdown >= dec!(0));
    }

    #[tokio::test]
    async fn test_leaderboard_with_losses_and_drawdown() {
        let pool = setup_test_db().await;
        let trader = "0x456";

        let now = Utc::now();

        // Simulate equity curve: +1000, +500, -800, +200 → peak 1500, trough 700 → drawdown ~53%
        sqlx::query(
            r#"
            INSERT INTO executed_trades
            (trader_address, closed_pnl, entry_px, exit_px, sz, is_long, timestamp)
            VALUES
            (?, 1000.0, 100.0, 110.0, 10.0, 1, ?),
            (?, 500.0, 110.0, 115.0, 10.0, 1, ?),
            (?, -800.0, 115.0, 107.0, 10.0, 0, ?),
            (?, 200.0, 107.0, 109.0, 10.0, 1, ?)
            "#
        )
        .bind(trader).bind(now - Duration::days(20))
        .bind(trader).bind(now - Duration::days(15))
        .bind(trader).bind(now - Duration::days(10))
        .bind(trader).bind(now - Duration::days(5))
        .execute(&pool)
        .await
        .unwrap();

        let metrics = calculate_trader_metrics(&pool, trader).await.unwrap();

        assert!(metrics.pnl_percent_30d > dec!(0)); // net positive
        assert!(metrics.win_rate == dec!(75.0)); // 3 wins, 1 loss
        assert!(metrics.max_drawdown > dec!(40.0)); // should detect ~53%
    }

    #[tokio::test]
    async fn test_no_trades_returns_zeros() {
        let pool = setup_test_db().await;
        let trader = "0x789";

        let metrics = calculate_trader_metrics(&pool, trader).await.unwrap();

        assert_eq!(metrics.pnl_percent_30d, dec!(0));
        assert_eq!(metrics.win_rate, dec!(0));
        assert_eq!(metrics.sharpe, dec!(0));
        assert_eq!(metrics.max_drawdown, dec!(0));
        assert_eq!(metrics.followers_count, 0);
        assert_eq!(metrics.volume_7d, dec!(0));
    }

    #[tokio::test]
    async fn test_followers_count() {
        let pool = setup_test_db().await;
        let trader = "0xfollowed";

        // Insert followers
        sqlx::query(
            "INSERT INTO followers (address) VALUES ('0xf1'), ('0xf2')"
        ).execute(&pool).await.unwrap();

        sqlx::query(
            "INSERT INTO copy_configs (follower_id, trader_address, is_active)
            VALUES (1, ?, 1), (2, ?, 1)"
        )
        .bind(trader)
        .bind(trader)
        .execute(&pool)
        .await
        .unwrap();

        let metrics = calculate_trader_metrics(&pool, trader).await.unwrap();

        assert_eq!(metrics.followers_count, 2);
    }

    #[tokio::test]
    async fn test_update_all_leaderboards() {
        let pool = setup_test_db().await;
        let trader1 = "0xt1";
        let trader2 = "0xt2";

        // Insert some trades for trader1
        sqlx::query(
            "INSERT INTO executed_trades
            (trader_address, closed_pnl, entry_px, exit_px, sz, is_long, timestamp)
            VALUES (?, 1000.0, 100.0, 110.0, 10.0, 1, DATETIME('now'))"
        )
        .bind(trader1)
        .execute(&pool)
        .await
        .unwrap();

        update_all_leaderboards(&pool).await.unwrap();

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM leaderboard")
            .fetch_one(&pool)
            .await
            .unwrap();

        assert!(count.0 >= 1); // at least one entry
    }
}
