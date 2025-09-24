#!/usr/bin/env python3
"""
Quick Performance Tests for Scribe Ledger

Simple, focused performance tests with immediate results.
"""

import requests
import time
import random
import string
import statistics
from tabulate import tabulate
from concurrent.futures import ThreadPoolExecutor
import sys


def generate_data(size_bytes: int) -> bytes:
    """Generate test data of specified size"""
    if size_bytes <= 1024:
        return ''.join(random.choices(string.ascii_letters, k=size_bytes)).encode()
    return b'A' * size_bytes  # Simple pattern for larger data


def test_write_speed(server_url: str, data_size: int, num_operations: int = 100):
    """Test write speed for given data size"""
    print(f"Testing WRITE speed - {format_bytes(data_size)} x {num_operations} operations...")
    
    latencies = []
    successes = 0
    
    start_time = time.time()
    
    for i in range(num_operations):
        key = f"perf_test_{int(time.time() * 1000000)}_{i}"
        data = generate_data(data_size)
        
        op_start = time.time()
        try:
            response = requests.put(
                f"{server_url}/{key}",
                data=data,
                headers={"Content-Type": "application/octet-stream"},
                timeout=30
            )
            op_end = time.time()
            latency_ms = (op_end - op_start) * 1000
            latencies.append(latency_ms)
            
            if response.status_code == 200:
                successes += 1
                
        except Exception as e:
            op_end = time.time() 
            latency_ms = (op_end - op_start) * 1000
            latencies.append(latency_ms)
    
    total_time = time.time() - start_time
    
    return {
        "operation": "WRITE",
        "data_size": data_size,
        "total_ops": num_operations,
        "successes": successes,
        "total_time": total_time,
        "ops_per_sec": num_operations / total_time,
        "throughput_mbps": (successes * data_size) / (total_time * 1024 * 1024),
        "avg_latency": statistics.mean(latencies) if latencies else 0,
        "min_latency": min(latencies) if latencies else 0,
        "max_latency": max(latencies) if latencies else 0,
        "p95_latency": percentile(latencies, 95) if latencies else 0
    }


def test_read_speed(server_url: str, keys: list, expected_size: int):
    """Test read speed using pre-existing keys"""
    if not keys:
        return None
        
    print(f"Testing READ speed - {format_bytes(expected_size)} x {len(keys)} operations...")
    
    latencies = []
    successes = 0
    total_bytes = 0
    
    start_time = time.time()
    
    for key in keys:
        op_start = time.time()
        try:
            response = requests.get(f"{server_url}/{key}", timeout=30)
            op_end = time.time()
            latency_ms = (op_end - op_start) * 1000
            latencies.append(latency_ms)
            
            if response.status_code == 200:
                successes += 1
                total_bytes += len(response.content)
                
        except Exception as e:
            op_end = time.time()
            latency_ms = (op_end - op_start) * 1000
            latencies.append(latency_ms)
    
    total_time = time.time() - start_time
    
    return {
        "operation": "READ",
        "data_size": expected_size,
        "total_ops": len(keys),
        "successes": successes,
        "total_time": total_time,
        "ops_per_sec": len(keys) / total_time,
        "throughput_mbps": total_bytes / (total_time * 1024 * 1024),
        "avg_latency": statistics.mean(latencies) if latencies else 0,
        "min_latency": min(latencies) if latencies else 0,
        "max_latency": max(latencies) if latencies else 0,
        "p95_latency": percentile(latencies, 95) if latencies else 0
    }


def test_concurrent_writes(server_url: str, data_size: int, num_threads: int = 10, ops_per_thread: int = 20):
    """Test concurrent write performance"""
    print(f"Testing CONCURRENT WRITES - {num_threads} threads x {ops_per_thread} ops each ({format_bytes(data_size)})...")
    
    def worker_write(thread_id):
        results = []
        for i in range(ops_per_thread):
            key = f"concurrent_{thread_id}_{i}_{int(time.time() * 1000000)}"
            data = generate_data(data_size)
            
            start = time.time()
            try:
                response = requests.put(
                    f"{server_url}/{key}",
                    data=data,
                    headers={"Content-Type": "application/octet-stream"},
                    timeout=30
                )
                latency = (time.time() - start) * 1000
                results.append((latency, response.status_code == 200, key))
            except:
                latency = (time.time() - start) * 1000
                results.append((latency, False, key))
        return results
    
    start_time = time.time()
    
    with ThreadPoolExecutor(max_workers=num_threads) as executor:
        futures = [executor.submit(worker_write, i) for i in range(num_threads)]
        all_results = []
        for future in futures:
            all_results.extend(future.result())
    
    total_time = time.time() - start_time
    
    latencies = [r[0] for r in all_results]
    successes = sum(1 for r in all_results if r[1])
    total_ops = len(all_results)
    
    return {
        "operation": "CONCURRENT_WRITE",
        "data_size": data_size,
        "total_ops": total_ops,
        "successes": successes,
        "total_time": total_time,
        "ops_per_sec": total_ops / total_time,
        "throughput_mbps": (successes * data_size) / (total_time * 1024 * 1024),
        "avg_latency": statistics.mean(latencies) if latencies else 0,
        "min_latency": min(latencies) if latencies else 0,
        "max_latency": max(latencies) if latencies else 0,
        "p95_latency": percentile(latencies, 95) if latencies else 0
    }


def percentile(data, p):
    """Calculate percentile"""
    if not data:
        return 0
    sorted_data = sorted(data)
    k = (len(sorted_data) - 1) * p / 100
    f = int(k)
    c = k - f
    if f + 1 < len(sorted_data):
        return sorted_data[f] * (1 - c) + sorted_data[f + 1] * c
    return sorted_data[f]


def format_bytes(bytes_val):
    """Format bytes in human readable form"""
    for unit in ['B', 'KB', 'MB', 'GB']:
        if bytes_val < 1024:
            return f"{bytes_val:.0f}{unit}"
        bytes_val /= 1024
    return f"{bytes_val:.1f}TB"


def main():
    server_url = "http://localhost:8080"
    if len(sys.argv) > 1:
        server_url = sys.argv[1]
    
    print("🚀 Scribe Ledger Quick Performance Test")
    print("=" * 50)
    print(f"Server: {server_url}")
    print()
    
    # Test health
    try:
        response = requests.get(f"{server_url}/health_check", timeout=5)
        print("✅ Server is responding")
    except:
        print("❌ Server not responding - make sure Scribe Ledger is running")
        return
    
    # Test scenarios
    test_sizes = [1024, 10240, 102400, 1048576]  # 1KB, 10KB, 100KB, 1MB
    results = []
    
    # Write tests
    for size in test_sizes:
        result = test_write_speed(server_url, size, 50)
        results.append(result)
    
    # Prepare keys for read tests
    print("\nPreparing data for read tests...")
    read_keys = {}
    for size in test_sizes:
        keys = []
        for i in range(30):
            key = f"read_test_{size}_{i}_{int(time.time() * 1000000)}"
            data = generate_data(size)
            try:
                response = requests.put(
                    f"{server_url}/{key}",
                    data=data,
                    headers={"Content-Type": "application/octet-stream"},
                    timeout=30
                )
                if response.status_code == 200:
                    keys.append(key)
            except:
                pass
        read_keys[size] = keys
    
    print()
    
    # Read tests
    for size in test_sizes:
        if read_keys[size]:
            result = test_read_speed(server_url, read_keys[size], size)
            if result:
                results.append(result)
    
    # Concurrent write test
    concurrent_result = test_concurrent_writes(server_url, 10240, 8, 25)  # 8 threads, 25 ops each
    results.append(concurrent_result)
    
    # Display results
    print("\n📊 PERFORMANCE RESULTS")
    print("=" * 70)
    
    table_data = []
    for r in results:
        table_data.append([
            r["operation"],
            format_bytes(r["data_size"]),
            f"{r['total_ops']:,}",
            f"{r['successes']:,}",
            f"{r['ops_per_sec']:.1f}",
            f"{r['throughput_mbps']:.2f}",
            f"{r['avg_latency']:.1f}",
            f"{r['p95_latency']:.1f}",
            f"{(r['successes']/r['total_ops']*100):.1f}%"
        ])
    
    headers = [
        "Operation", "Data Size", "Total Ops", "Success", "Ops/Sec", 
        "MB/s", "Avg Lat(ms)", "P95 Lat(ms)", "Success%"
    ]
    
    print(tabulate(table_data, headers=headers, tablefmt="grid"))
    
    # Summary
    print("\n📈 SUMMARY")
    total_ops = sum(r["total_ops"] for r in results)
    avg_throughput = statistics.mean([r["throughput_mbps"] for r in results if r["throughput_mbps"] > 0])
    avg_latency = statistics.mean([r["avg_latency"] for r in results if r["avg_latency"] > 0])
    
    print(f"Total Operations: {total_ops:,}")
    print(f"Average Throughput: {avg_throughput:.2f} MB/s")
    print(f"Average Latency: {avg_latency:.1f} ms")
    print("\n✅ Performance test completed!")


if __name__ == "__main__":
    main()