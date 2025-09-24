#!/usr/bin/env python3
"""
Scribe Ledger Performance Benchmark Framework

This script provides comprehensive performance testing for Scribe Ledger,
measuring read/write speeds across various scenarios and displaying results
in formatted tables.
"""

import asyncio
import time
import random
import string
import statistics
import subprocess
import signal
import sys
import os
import json
from typing import List, Dict, Tuple, Optional
from dataclasses import dataclass, asdict
from concurrent.futures import ThreadPoolExecutor, as_completed
from pathlib import Path
import requests
from tabulate import tabulate


@dataclass
class BenchmarkResult:
    """Results from a benchmark test"""
    test_name: str
    operation: str  # "write" or "read"
    total_operations: int
    data_size_bytes: int
    duration_seconds: float
    ops_per_second: float
    throughput_mbps: float
    min_latency_ms: float
    max_latency_ms: float
    avg_latency_ms: float
    p50_latency_ms: float
    p95_latency_ms: float
    p99_latency_ms: float
    success_rate: float
    errors: int


@dataclass
class BenchmarkConfig:
    """Configuration for benchmark tests"""
    server_url: str = "http://localhost:8080"
    warmup_operations: int = 100
    test_duration_seconds: int = 30
    max_concurrent_requests: int = 10
    data_sizes: List[int] = None
    custom_payloads: Dict[str, bytes] = None
    
    def __post_init__(self):
        if self.data_sizes is None:
            self.data_sizes = [1024, 10240, 102400, 1048576]  # 1KB, 10KB, 100KB, 1MB
        if self.custom_payloads is None:
            self.custom_payloads = {}


class ScribeBenchmark:
    """Main benchmark runner for Scribe Ledger"""
    
    def __init__(self, config: BenchmarkConfig):
        self.config = config
        self.session = requests.Session()
        self.session.timeout = 30
        self.results: List[BenchmarkResult] = []
        
    def generate_test_data(self, size_bytes: int) -> bytes:
        """Generate random test data of specified size"""
        if size_bytes <= 1024:
            # For small data, use random string
            return ''.join(random.choices(string.ascii_letters + string.digits, k=size_bytes)).encode()
        else:
            # For larger data, use random bytes (more efficient)
            return os.urandom(size_bytes)
    
    def generate_test_key(self, prefix: str = "bench") -> str:
        """Generate unique test key"""
        timestamp = int(time.time() * 1000000)  # microseconds
        random_suffix = ''.join(random.choices(string.ascii_lowercase, k=8))
        return f"{prefix}_{timestamp}_{random_suffix}"
    
    def measure_write_operation(self, key: str, data: bytes) -> Tuple[float, bool]:
        """Measure single write operation, return (latency_ms, success)"""
        start_time = time.time()
        try:
            response = self.session.put(
                f"{self.config.server_url}/{key}",
                data=data,
                headers={"Content-Type": "application/octet-stream"}
            )
            latency_ms = (time.time() - start_time) * 1000
            return latency_ms, response.status_code == 200
        except Exception as e:
            latency_ms = (time.time() - start_time) * 1000
            return latency_ms, False
    
    def measure_read_operation(self, key: str) -> Tuple[float, bool, int]:
        """Measure single read operation, return (latency_ms, success, data_size)"""
        start_time = time.time()
        try:
            response = self.session.get(f"{self.config.server_url}/{key}")
            latency_ms = (time.time() - start_time) * 1000
            success = response.status_code == 200
            data_size = len(response.content) if success else 0
            return latency_ms, success, data_size
        except Exception as e:
            latency_ms = (time.time() - start_time) * 1000
            return latency_ms, False, 0
    
    def run_write_benchmark(self, test_name: str, data_size: int, duration_seconds: int) -> BenchmarkResult:
        """Run write performance benchmark"""
        print(f"🏃 Running {test_name} (Write, {self._format_bytes(data_size)})...")
        
        latencies = []
        successful_ops = 0
        total_ops = 0
        start_time = time.time()
        end_time = start_time + duration_seconds
        
        # Warmup
        print("  Warming up...")
        for _ in range(self.config.warmup_operations):
            key = self.generate_test_key("warmup")
            data = self.generate_test_data(data_size)
            self.measure_write_operation(key, data)
        
        print("  Running benchmark...")
        with ThreadPoolExecutor(max_workers=self.config.max_concurrent_requests) as executor:
            futures = []
            
            while time.time() < end_time:
                if len(futures) < self.config.max_concurrent_requests:
                    key = self.generate_test_key("write")
                    data = self.generate_test_data(data_size)
                    future = executor.submit(self.measure_write_operation, key, data)
                    futures.append(future)
                
                # Collect completed operations
                completed_futures = []
                for future in futures:
                    if future.done():
                        completed_futures.append(future)
                        latency_ms, success = future.result()
                        latencies.append(latency_ms)
                        total_ops += 1
                        if success:
                            successful_ops += 1
                
                # Remove completed futures
                for future in completed_futures:
                    futures.remove(future)
                
                # Small delay to prevent overwhelming
                time.sleep(0.001)
            
            # Wait for remaining operations
            for future in futures:
                latency_ms, success = future.result()
                latencies.append(latency_ms)
                total_ops += 1
                if success:
                    successful_ops += 1
        
        actual_duration = time.time() - start_time
        return self._calculate_result(test_name, "write", total_ops, data_size, 
                                    actual_duration, latencies, successful_ops)
    
    def run_read_benchmark(self, test_name: str, keys: List[str], expected_data_size: int, 
                          duration_seconds: int) -> BenchmarkResult:
        """Run read performance benchmark"""
        print(f"🏃 Running {test_name} (Read, {self._format_bytes(expected_data_size)})...")
        
        if not keys:
            print("  ⚠️  No keys available for read benchmark")
            return self._empty_result(test_name, "read", expected_data_size)
        
        latencies = []
        successful_ops = 0
        total_ops = 0
        start_time = time.time()
        end_time = start_time + duration_seconds
        
        print("  Running benchmark...")
        with ThreadPoolExecutor(max_workers=self.config.max_concurrent_requests) as executor:
            futures = []
            
            while time.time() < end_time:
                if len(futures) < self.config.max_concurrent_requests:
                    key = random.choice(keys)
                    future = executor.submit(self.measure_read_operation, key)
                    futures.append(future)
                
                # Collect completed operations
                completed_futures = []
                for future in futures:
                    if future.done():
                        completed_futures.append(future)
                        latency_ms, success, _ = future.result()
                        latencies.append(latency_ms)
                        total_ops += 1
                        if success:
                            successful_ops += 1
                
                # Remove completed futures
                for future in completed_futures:
                    futures.remove(future)
                
                time.sleep(0.001)
            
            # Wait for remaining operations
            for future in futures:
                latency_ms, success, _ = future.result()
                latencies.append(latency_ms)
                total_ops += 1
                if success:
                    successful_ops += 1
        
        actual_duration = time.time() - start_time
        return self._calculate_result(test_name, "read", total_ops, expected_data_size,
                                    actual_duration, latencies, successful_ops)
    
    def run_mixed_benchmark(self, test_name: str, data_size: int, duration_seconds: int,
                           read_ratio: float = 0.7) -> Tuple[BenchmarkResult, BenchmarkResult]:
        """Run mixed read/write benchmark"""
        print(f"🏃 Running {test_name} (Mixed {int(read_ratio*100)}% read, {self._format_bytes(data_size)})...")
        
        # Pre-populate some keys for reading
        keys = []
        print("  Pre-populating data...")
        for i in range(100):
            key = self.generate_test_key("mixed")
            data = self.generate_test_data(data_size)
            _, success = self.measure_write_operation(key, data)
            if success:
                keys.append(key)
        
        print(f"  Pre-populated {len(keys)} keys")
        
        read_latencies = []
        write_latencies = []
        read_successes = 0
        write_successes = 0
        total_reads = 0
        total_writes = 0
        
        start_time = time.time()
        end_time = start_time + duration_seconds
        
        print("  Running mixed benchmark...")
        with ThreadPoolExecutor(max_workers=self.config.max_concurrent_requests) as executor:
            futures = []
            
            while time.time() < end_time:
                if len(futures) < self.config.max_concurrent_requests:
                    if random.random() < read_ratio and keys:
                        # Read operation
                        key = random.choice(keys)
                        future = executor.submit(self._measure_mixed_read, key)
                        futures.append(("read", future))
                    else:
                        # Write operation
                        key = self.generate_test_key("mixed")
                        data = self.generate_test_data(data_size)
                        future = executor.submit(self._measure_mixed_write, key, data)
                        futures.append(("write", future))
                
                # Collect completed operations
                completed_futures = []
                for op_type, future in futures:
                    if future.done():
                        completed_futures.append((op_type, future))
                        latency_ms, success = future.result()
                        
                        if op_type == "read":
                            read_latencies.append(latency_ms)
                            total_reads += 1
                            if success:
                                read_successes += 1
                        else:
                            write_latencies.append(latency_ms)
                            total_writes += 1
                            if success:
                                write_successes += 1
                                # Add successful writes to keys for future reads
                                if len(keys) < 1000:  # Limit key pool size
                                    keys.append(future.result()[2] if len(future.result()) > 2 else None)
                
                # Remove completed futures
                for item in completed_futures:
                    futures.remove(item)
                
                time.sleep(0.001)
            
            # Wait for remaining operations
            for op_type, future in futures:
                latency_ms, success = future.result()
                if op_type == "read":
                    read_latencies.append(latency_ms)
                    total_reads += 1
                    if success:
                        read_successes += 1
                else:
                    write_latencies.append(latency_ms)
                    total_writes += 1
                    if success:
                        write_successes += 1
        
        actual_duration = time.time() - start_time
        
        read_result = self._calculate_result(f"{test_name}_read", "read", total_reads, 
                                           data_size, actual_duration, read_latencies, read_successes)
        write_result = self._calculate_result(f"{test_name}_write", "write", total_writes,
                                            data_size, actual_duration, write_latencies, write_successes)
        
        return read_result, write_result
    
    def _measure_mixed_read(self, key: str) -> Tuple[float, bool]:
        """Helper for mixed benchmark read"""
        latency_ms, success, _ = self.measure_read_operation(key)
        return latency_ms, success
    
    def _measure_mixed_write(self, key: str, data: bytes) -> Tuple[float, bool, str]:
        """Helper for mixed benchmark write"""
        latency_ms, success = self.measure_write_operation(key, data)
        return latency_ms, success, key if success else None
    
    def _calculate_result(self, test_name: str, operation: str, total_ops: int, 
                         data_size: int, duration: float, latencies: List[float], 
                         successful_ops: int) -> BenchmarkResult:
        """Calculate benchmark result from measurements"""
        if not latencies:
            return self._empty_result(test_name, operation, data_size)
        
        ops_per_second = total_ops / duration
        throughput_mbps = (successful_ops * data_size) / (duration * 1024 * 1024)
        success_rate = (successful_ops / total_ops) * 100 if total_ops > 0 else 0
        
        latencies.sort()
        return BenchmarkResult(
            test_name=test_name,
            operation=operation,
            total_operations=total_ops,
            data_size_bytes=data_size,
            duration_seconds=duration,
            ops_per_second=ops_per_second,
            throughput_mbps=throughput_mbps,
            min_latency_ms=min(latencies),
            max_latency_ms=max(latencies),
            avg_latency_ms=statistics.mean(latencies),
            p50_latency_ms=self._percentile(latencies, 50),
            p95_latency_ms=self._percentile(latencies, 95),
            p99_latency_ms=self._percentile(latencies, 99),
            success_rate=success_rate,
            errors=total_ops - successful_ops
        )
    
    def _empty_result(self, test_name: str, operation: str, data_size: int) -> BenchmarkResult:
        """Create empty result for failed tests"""
        return BenchmarkResult(
            test_name=test_name, operation=operation, total_operations=0,
            data_size_bytes=data_size, duration_seconds=0, ops_per_second=0,
            throughput_mbps=0, min_latency_ms=0, max_latency_ms=0, avg_latency_ms=0,
            p50_latency_ms=0, p95_latency_ms=0, p99_latency_ms=0, success_rate=0, errors=0
        )
    
    def _percentile(self, data: List[float], percentile: int) -> float:
        """Calculate percentile of sorted data"""
        if not data:
            return 0
        k = (len(data) - 1) * percentile / 100
        f = int(k)
        c = k - f
        if f + 1 < len(data):
            return data[f] * (1 - c) + data[f + 1] * c
        return data[f]
    
    def _format_bytes(self, bytes_val: int) -> str:
        """Format bytes in human readable form"""
        for unit in ['B', 'KB', 'MB', 'GB']:
            if bytes_val < 1024:
                return f"{bytes_val:.1f}{unit}"
            bytes_val /= 1024
        return f"{bytes_val:.1f}TB"
    
    def check_server_health(self) -> bool:
        """Check if server is running and healthy"""
        try:
            # Try to do a simple operation
            test_key = "health_check"
            test_data = b"health"
            response = self.session.put(
                f"{self.config.server_url}/{test_key}",
                data=test_data,
                timeout=5
            )
            return response.status_code == 200
        except:
            return False
    
    def run_all_benchmarks(self):
        """Run comprehensive benchmark suite"""
        print("🚀 Starting Scribe Ledger Performance Benchmarks")
        print("=" * 60)
        
        # Check server health
        if not self.check_server_health():
            print("❌ Server health check failed. Is Scribe Ledger running?")
            print(f"   Expected server at: {self.config.server_url}")
            return
        
        print("✅ Server health check passed")
        print(f"📊 Configuration:")
        print(f"   Server: {self.config.server_url}")
        print(f"   Test duration: {self.config.test_duration_seconds}s per test")
        print(f"   Max concurrent requests: {self.config.max_concurrent_requests}")
        print(f"   Warmup operations: {self.config.warmup_operations}")
        print()
        
        # Run write benchmarks for different data sizes
        write_keys_by_size = {}
        for data_size in self.config.data_sizes:
            result = self.run_write_benchmark(f"Write_{self._format_bytes(data_size)}", 
                                            data_size, self.config.test_duration_seconds)
            self.results.append(result)
            
            # Collect keys for read tests (store up to 500 keys per size)
            keys = []
            for _ in range(min(500, int(result.successful_operations * 0.1))):
                key = self.generate_test_key("read_prep")
                data = self.generate_test_data(data_size)
                _, success = self.measure_write_operation(key, data)
                if success:
                    keys.append(key)
            write_keys_by_size[data_size] = keys
        
        print()
        
        # Run read benchmarks
        for data_size in self.config.data_sizes:
            keys = write_keys_by_size.get(data_size, [])
            result = self.run_read_benchmark(f"Read_{self._format_bytes(data_size)}", 
                                           keys, data_size, self.config.test_duration_seconds)
            self.results.append(result)
        
        print()
        
        # Run mixed benchmarks
        for data_size in [1024, 102400]:  # 1KB and 100KB for mixed tests
            read_result, write_result = self.run_mixed_benchmark(
                f"Mixed_{self._format_bytes(data_size)}", data_size, 
                self.config.test_duration_seconds, read_ratio=0.7
            )
            self.results.append(read_result)
            self.results.append(write_result)
        
        print()
        self.display_results()
    
    def display_results(self):
        """Display benchmark results in formatted tables"""
        print("📈 BENCHMARK RESULTS")
        print("=" * 80)
        
        # Separate results by operation type
        write_results = [r for r in self.results if r.operation == "write"]
        read_results = [r for r in self.results if r.operation == "read"]
        
        # Write Results Table
        if write_results:
            print("\n🔥 WRITE PERFORMANCE")
            write_table = []
            for r in write_results:
                write_table.append([
                    r.test_name.replace("_", " "),
                    self._format_bytes(r.data_size_bytes),
                    f"{r.total_operations:,}",
                    f"{r.ops_per_second:.1f}",
                    f"{r.throughput_mbps:.2f}",
                    f"{r.avg_latency_ms:.1f}",
                    f"{r.p95_latency_ms:.1f}",
                    f"{r.success_rate:.1f}%"
                ])
            
            print(tabulate(write_table, headers=[
                "Test", "Data Size", "Operations", "Ops/Sec", "MB/s", 
                "Avg Latency (ms)", "P95 Latency (ms)", "Success Rate"
            ], tablefmt="grid"))
        
        # Read Results Table
        if read_results:
            print("\n📖 READ PERFORMANCE")
            read_table = []
            for r in read_results:
                read_table.append([
                    r.test_name.replace("_", " "),
                    self._format_bytes(r.data_size_bytes),
                    f"{r.total_operations:,}",
                    f"{r.ops_per_second:.1f}",
                    f"{r.throughput_mbps:.2f}",
                    f"{r.avg_latency_ms:.1f}",
                    f"{r.p95_latency_ms:.1f}",
                    f"{r.success_rate:.1f}%"
                ])
            
            print(tabulate(read_table, headers=[
                "Test", "Data Size", "Operations", "Ops/Sec", "MB/s",
                "Avg Latency (ms)", "P95 Latency (ms)", "Success Rate"
            ], tablefmt="grid"))
        
        # Summary Statistics
        print("\n📊 SUMMARY STATISTICS")
        if self.results:
            all_ops = sum(r.total_operations for r in self.results)
            avg_throughput = statistics.mean([r.throughput_mbps for r in self.results if r.throughput_mbps > 0])
            avg_latency = statistics.mean([r.avg_latency_ms for r in self.results if r.avg_latency_ms > 0])
            overall_success = statistics.mean([r.success_rate for r in self.results])
            
            summary_table = [
                ["Total Operations", f"{all_ops:,}"],
                ["Average Throughput", f"{avg_throughput:.2f} MB/s"],
                ["Average Latency", f"{avg_latency:.1f} ms"],
                ["Overall Success Rate", f"{overall_success:.1f}%"],
                ["Total Test Duration", f"{len(self.results) * self.config.test_duration_seconds}s"]
            ]
            
            print(tabulate(summary_table, headers=["Metric", "Value"], tablefmt="grid"))
        
        print("\n✅ Benchmark completed!")
    
    def save_results_json(self, filename: str = "benchmark_results.json"):
        """Save results to JSON file"""
        results_data = {
            "timestamp": time.time(),
            "config": asdict(self.config),
            "results": [asdict(r) for r in self.results]
        }
        
        with open(filename, 'w') as f:
            json.dump(results_data, f, indent=2)
        
        print(f"💾 Results saved to {filename}")


def main():
    """Main benchmark runner"""
    print("🏁 Scribe Ledger Performance Benchmark")
    print("=" * 50)
    
    # Default configuration
    config = BenchmarkConfig(
        server_url="http://localhost:8080",
        warmup_operations=50,
        test_duration_seconds=15,
        max_concurrent_requests=20,
        data_sizes=[1024, 10240, 102400, 1048576, 5242880]  # 1KB to 5MB
    )
    
    # Allow command line override
    if len(sys.argv) > 1:
        config.server_url = sys.argv[1]
    if len(sys.argv) > 2:
        config.test_duration_seconds = int(sys.argv[2])
    if len(sys.argv) > 3:
        config.max_concurrent_requests = int(sys.argv[3])
    
    benchmark = ScribeBenchmark(config)
    
    try:
        benchmark.run_all_benchmarks()
        benchmark.save_results_json(f"benchmark_{int(time.time())}.json")
    except KeyboardInterrupt:
        print("\n⚠️  Benchmark interrupted by user")
        if benchmark.results:
            benchmark.display_results()
    except Exception as e:
        print(f"❌ Benchmark failed: {e}")
        sys.exit(1)


if __name__ == "__main__":
    main()