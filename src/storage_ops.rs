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
        for i in 0..ops {
            ledger.put(&keys[i], &values[i])?;
        }
    } else {
        // Use batching for better performance
        let batch_size = std::cmp::min(OPTIMAL_BATCH_SIZE, ops / 4);
        let mut i = 0;
        while i < ops {
            let mut batch = SimpleScribeLedger::new_batch();
            let end = std::cmp::min(i + batch_size, ops);

            for j in i..end {
                batch.insert(keys[j].as_bytes(), values[j].as_bytes());
            }

            ledger.apply_batch(batch)?;
            i = end;
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
        for i in 0..put_ops {
            ledger.put(&keys[i], &values[i])?;
        }
    } else {
        let batch_size = std::cmp::min(50, put_ops / 4);
        let mut i = 0;
        while i < put_ops {
            let mut batch = SimpleScribeLedger::new_batch();
            let end = std::cmp::min(i + batch_size, put_ops);

            for j in i..end {
                batch.insert(keys[j].as_bytes(), values[j].as_bytes());
            }

            ledger.apply_batch(batch)?;
            i = end;
        }
    }

    // GET operations
    for i in 0..put_ops {
        let _ = ledger.get(&keys[i])?;
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

    // Use optimal batching
    let batch_size = OPTIMAL_BATCH_SIZE;
    let mut i = 0;
    while i < 10000 {
        let mut batch = SimpleScribeLedger::new_batch();
        let end = std::cmp::min(i + batch_size, 10000);

        for j in i..end {
            batch.insert(keys[j].as_bytes(), values[j].as_bytes());
        }

        ledger.apply_batch(batch)?;
        i = end;
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
        for i in 0..ops {
            ledger.put(&keys[i], &values[i])?;
        }
    } else {
        let batch_size = std::cmp::min(OPTIMAL_BATCH_SIZE, ops / 4);
        let mut i = 0;
        while i < ops {
            let mut batch = SimpleScribeLedger::new_batch();
            let end = std::cmp::min(i + batch_size, ops);

            for j in i..end {
                batch.insert(keys[j].as_bytes(), values[j].as_bytes());
            }

            ledger.apply_batch(batch)?;
            i = end;
        }
    }

    ledger.flush()?;
    Ok(())
}
