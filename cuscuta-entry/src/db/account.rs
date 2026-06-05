use sqlx::{Postgres, Transaction};

pub async fn count_active_account(
    mut tx: Transaction<'_, Postgres>,
) -> Result<usize, sqlx::Error> {
    let row: i64 = sqlx::query_scalar(
        r"
        SELECT COUNT(*) 
        FROM account_table
        WHERE state = 'Using';
        ",
    )
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    #[allow(clippy::cast_possible_truncation)]
    Ok(row.cast_unsigned() as usize)
}
