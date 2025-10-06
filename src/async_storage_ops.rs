use crate::storage::{SledStorage, StorageBackend};
use anyhow::Result;

/// Optimized batch size for async storage operations
const OPTIMAL_BATCH_SIZE: usize = 50;

/// Perform optimized async PUT operations with batching
///
/// # Arguments
/// * `storage` - The storage backend
/// * `keys` - Slice of keys to put
/// * `values` - Slice of values corresponding to keys
///
/// # Returns
/// Result indicating success or failure
pub async fn batched_async_put_operations(
    storage: &SledStorage,
    keys: &[Vec<u8>],
    values: &[Vec<u8>],
) -> Result<()> {
    let ops = keys.len();
    
    // For small batches, do them sequentially
    if ops <= OPTIMAL_BATCH_SIZE {
        for i in 0..ops {
            storage.put(keys[i].clone(), values[i].clone()).await?;
        }
    } else {
        // For larger batches, process in chunks
        let mut i = 0;
        while i < ops {
            let end = std::cmp::min(i + OPTIMAL_BATCH_SIZE, ops);
            
            for j in i..end {
                storage.put(keys[j].clone(), values[j].clone()).await?;
            }
            
            i = end;
        }
    }
    
    storage.flush().await?;
    Ok(())
}

/// Perform optimized async GET operations
///
/// # Arguments
/// * `storage` - The storage backend
/// * `keys` - Slice of keys to get
///
/// # Returns
/// Result indicating success or failure
pub async fn batched_async_get_operations(
    storage: &SledStorage,
    keys: &[Vec<u8>],
) -> Result<()> {
    for key in keys {
        let _ = storage.get(key).await?;
    }
    Ok(())
}

/// Perform optimized async mixed PUT/GET operations
///
/// # Arguments
/// * `storage` - The storage backend
/// * `keys` - Slice of keys to use
/// * `values` - Slice of values corresponding to keys
///
/// # Returns
/// Result indicating success or failure
pub async fn batched_async_mixed_operations(
    storage: &SledStorage,
    keys: &[Vec<u8>],
    values: &[Vec<u8>],
) -> Result<()> {
    let put_ops = keys.len() / 2;

    // PUT operations (first half)
    for i in 0..put_ops {
        storage.put(keys[i].clone(), values[i].clone()).await?;
    }

    // GET operations
    for i in 0..put_ops {
        let _ = storage.get(&keys[i]).await?;
    }

    storage.flush().await?;
    Ok(())
}

/// Populate async storage with data
///
/// # Arguments
/// * `storage` - The storage backend
/// * `keys` - Slice of keys to put
/// * `values` - Slice of values corresponding to keys
///
/// # Returns
/// Result indicating success or failure
pub async fn populate_async_storage(
    storage: &SledStorage,
    keys: &[Vec<u8>],
    values: &[Vec<u8>],
) -> Result<()> {
    let ops = keys.len();
    
    for i in 0..ops {
        storage.put(keys[i].clone(), values[i].clone()).await?;
    }
    
    storage.flush().await?;
    Ok(())
}

/// Perform concurrent async operations using tokio::spawn
///
/// # Arguments
/// * `storage` - Arc-wrapped storage backend
/// * `concurrent` - Number of concurrent operations
///
/// # Returns
/// Result indicating success or failure
pub async fn concurrent_async_operations(
    storage: std::sync::Arc<SledStorage>,
    concurrent: usize,
) -> Result<()> {
    let mut handles = vec![];

    for i in 0..concurrent {
        let storage_clone = std::sync::Arc::clone(&storage);
        let handle = tokio::spawn(async move {
            let key = format!("key{}", i).into_bytes();
            let value = format!("value{}", i).into_bytes();
            storage_clone.put(key.clone(), value.clone()).await.unwrap();
            let _result = storage_clone.get(&key).await.unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await?;
    }

    Ok(())
}
