use crate::SimpleScribeLedger;
use anyhow::Result;

/// Optimized batch size for storage operations
const OPTIMAL_BATCH_SIZE: usize = 100;

/// Perform optimized PUT operations with automatic batching
///
/// # Arguments
/// * `ledger` - The ledger instance
/// * `keys` - Slice of keys to put
/// * `values` - Slice of values corresponding to keys
/// * `use_warmup` - Whether to perform a warmup operation first
///
/// # Returns
/// Result indicating success or failure
pub fn batched_put_operations(
    ledger: &SimpleScribeLedger,
    keys: &[String],
    values: &[String],
    use_warmup: bool,
) -> Result<()> {
    if use_warmup {
        ledger.put("warmup", "value")?;
    }

    let ops = keys.len();

    if ops <= 10 {
        // For small operations, use individual puts
        for (key, value) in keys.iter().zip(values.iter()).take(ops) {
            ledger.put(key, value)?;
        }
    } else {
        // Use batching for better performance with optimal batch size
        let batch_size = (ops / 4).clamp(10, OPTIMAL_BATCH_SIZE);

        for chunk_start in (0..ops).step_by(batch_size) {
            let chunk_end = std::cmp::min(chunk_start + batch_size, ops);
            let mut batch = SimpleScribeLedger::new_batch();

            for j in chunk_start..chunk_end {
                batch.insert(keys[j].as_bytes(), values[j].as_bytes());
            }

            ledger.apply_batch(batch)?;
        }
    }

    Ok(())
}

/// Perform optimized GET operations
///
/// # Arguments
/// * `ledger` - The ledger instance
/// * `keys` - Slice of keys to get
///
/// # Returns
/// Result indicating success or failure
pub fn batched_get_operations(ledger: &SimpleScribeLedger, keys: &[String]) -> Result<()> {
    for key in keys {
        let _ = ledger.get(key)?;
    }
    Ok(())
}

/// Perform optimized mixed PUT/GET operations with automatic batching
///
/// # Arguments
/// * `ledger` - The ledger instance
/// * `keys` - Slice of keys to use
/// * `values` - Slice of values corresponding to keys
/// * `use_warmup` - Whether to perform a warmup operation first
///
/// # Returns
/// Result indicating success or failure
pub fn batched_mixed_operations(
    ledger: &SimpleScribeLedger,
    keys: &[String],
    values: &[String],
    use_warmup: bool,
) -> Result<()> {
    if use_warmup {
        ledger.put("warmup", "value")?;
    }

    let put_ops = keys.len() / 2;

    // PUT operations
    if put_ops <= 10 {
        for (key, value) in keys.iter().zip(values.iter()).take(put_ops) {
            ledger.put(key, value)?;
        }
    } else {
        let batch_size = (put_ops / 4).clamp(10, 50);

        for chunk_start in (0..put_ops).step_by(batch_size) {
            let chunk_end = std::cmp::min(chunk_start + batch_size, put_ops);
            let mut batch = SimpleScribeLedger::new_batch();

            for j in chunk_start..chunk_end {
                batch.insert(keys[j].as_bytes(), values[j].as_bytes());
            }

            ledger.apply_batch(batch)?;
        }
    }

    // GET operations
    for key in keys.iter().take(put_ops) {
        let _ = ledger.get(key)?;
    }

    Ok(())
}

/// Perform optimized throughput PUT test with 10000 operations
///
/// # Arguments
/// * `ledger` - The ledger instance
/// * `keys` - Pre-allocated keys
/// * `values` - Pre-allocated values
///
/// # Returns
/// Result indicating success or failure
pub fn throughput_put_10k(
    ledger: &SimpleScribeLedger,
    keys: &[String],
    values: &[String],
) -> Result<()> {
    // Warmup
    ledger.put("warmup", "value")?;

    // Use optimal batching with step_by for cleaner code
    for chunk_start in (0..10000).step_by(OPTIMAL_BATCH_SIZE) {
        let chunk_end = std::cmp::min(chunk_start + OPTIMAL_BATCH_SIZE, 10000);
        let mut batch = SimpleScribeLedger::new_batch();

        for j in chunk_start..chunk_end {
            batch.insert(keys[j].as_bytes(), values[j].as_bytes());
        }

        ledger.apply_batch(batch)?;
    }

    ledger.flush()?;
    Ok(())
}

/// Perform optimized throughput GET test with 10000 operations
///
/// # Arguments
/// * `ledger` - The ledger instance (pre-populated)
/// * `keys` - Pre-allocated keys
///
/// # Returns
/// Result indicating success or failure
pub fn throughput_get_10k(ledger: &SimpleScribeLedger, keys: &[String]) -> Result<()> {
    for key in keys {
        let _ = ledger.get(key)?;
    }
    Ok(())
}

/// Populate ledger with data using optimized batching
///
/// # Arguments
/// * `ledger` - The ledger instance
/// * `keys` - Slice of keys to put
/// * `values` - Slice of values corresponding to keys
/// * `use_warmup` - Whether to perform a warmup operation first
///
/// # Returns
/// Result indicating success or failure
pub fn populate_ledger(
    ledger: &SimpleScribeLedger,
    keys: &[String],
    values: &[String],
    use_warmup: bool,
) -> Result<()> {
    if use_warmup {
        ledger.put("warmup", "value")?;
    }

    let ops = keys.len();

    if ops <= 10 {
        for (key, value) in keys.iter().zip(values.iter()).take(ops) {
            ledger.put(key, value)?;
        }
    } else {
        let batch_size = (ops / 4).clamp(10, OPTIMAL_BATCH_SIZE);

        for chunk_start in (0..ops).step_by(batch_size) {
            let chunk_end = std::cmp::min(chunk_start + batch_size, ops);
            let mut batch = SimpleScribeLedger::new_batch();

            for j in chunk_start..chunk_end {
                batch.insert(keys[j].as_bytes(), values[j].as_bytes());
            }

            ledger.apply_batch(batch)?;
        }
    }

    ledger.flush()?;
    Ok(())
}
