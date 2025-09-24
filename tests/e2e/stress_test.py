#!/usr/bin/env python3
"""
Stress Test for Scribe Ledger

High-load testing to evaluate system limits and stability.
"""

import requests
import time
import threading
import random
import string
import queue
import sys
from tabulate import tabulate
from concurrent.futures import ThreadPoolExecutor, as_completed
import signal


class StressTestRunner:
    def __init__(self, server_url="http://localhost:8080"):
        self.server_url = server_url
        self.results_queue = queue.Queue()
        self.stop_event = threading.Event()
        self.stats = {
            'total_operations': 0,
            'successful_operations': 0,
            'failed_operations': 0,
            'total_bytes_written': 0,
            'total_bytes_read': 0,
            'min_latency': float('inf'),
            'max_latency': 0,
            'latency_sum': 0,
            'start_time': None,
            'errors': []
        }
        
    def generate_random_data(self, min_size=1024, max_size=102400):
        """Generate random data between min and max size"""
        size = random.randint(min_size, max_size)
        return ''.join(random.choices(string.ascii_letters + string.digits + '\n\t ', k=size)).encode()
    
    def stress_write_worker(self, worker_id, duration_seconds, operations_per_second=10):
        """Worker thread for stress writing"""
        operations = 0
        start_time = time.time()
        
        while not self.stop_event.is_set() and (time.time() - start_time) < duration_seconds:
            try:
                # Generate test data
                key = f"stress_{worker_id}_{operations}_{int(time.time() * 1000000)}"
                data = self.generate_random_data()
                
                # Measure operation
                op_start = time.time()
                response = requests.put(
                    f"{self.server_url}/{key}",
                    data=data,
                    headers={"Content-Type": "application/octet-stream"},
                    timeout=10
                )
                op_duration = time.time() - op_start
                
                # Record results
                success = response.status_code == 200
                self.results_queue.put({
                    'operation': 'write',
                    'success': success,
                    'latency': op_duration,
                    'data_size': len(data),
                    'worker_id': worker_id,
                    'error': None if success else f"HTTP {response.status_code}"
                })
                
                operations += 1
                
                # Rate limiting
                if operations_per_second > 0:
                    time.sleep(1.0 / operations_per_second)
                    
            except Exception as e:
                op_duration = time.time() - op_start if 'op_start' in locals() else 0
                self.results_queue.put({
                    'operation': 'write',
                    'success': False,
                    'latency': op_duration,
                    'data_size': 0,
                    'worker_id': worker_id,
                    'error': str(e)
                })
                time.sleep(0.1)  # Brief pause on error
        
        print(f"  Worker {worker_id}: Completed {operations} operations")
    
    def stress_read_worker(self, worker_id, keys, duration_seconds, operations_per_second=20):
        """Worker thread for stress reading"""
        if not keys:
            return
            
        operations = 0
        start_time = time.time()
        
        while not self.stop_event.is_set() and (time.time() - start_time) < duration_seconds:
            try:
                # Select random key
                key = random.choice(keys)
                
                # Measure operation
                op_start = time.time()
                response = requests.get(f"{self.server_url}/{key}", timeout=10)
                op_duration = time.time() - op_start
                
                # Record results
                success = response.status_code == 200
                data_size = len(response.content) if success else 0
                
                self.results_queue.put({
                    'operation': 'read',
                    'success': success,
                    'latency': op_duration,
                    'data_size': data_size,
                    'worker_id': worker_id,
                    'error': None if success else f"HTTP {response.status_code}"
                })
                
                operations += 1
                
                # Rate limiting
                if operations_per_second > 0:
                    time.sleep(1.0 / operations_per_second)
                    
            except Exception as e:
                op_duration = time.time() - op_start if 'op_start' in locals() else 0
                self.results_queue.put({
                    'operation': 'read',
                    'success': False,
                    'latency': op_duration,
                    'data_size': 0,
                    'worker_id': worker_id,
                    'error': str(e)
                })
                time.sleep(0.1)
        
        print(f"  Reader {worker_id}: Completed {operations} operations")
    
    def monitor_progress(self, duration_seconds):
        """Monitor and display progress during stress test"""
        start_time = time.time()
        last_report = start_time
        last_total = 0
        
        while not self.stop_event.is_set() and (time.time() - start_time) < duration_seconds:
            # Process results from queue
            try:
                while True:
                    result = self.results_queue.get_nowait()
                    self.update_stats(result)
            except queue.Empty:
                pass
            
            # Report progress every 5 seconds
            current_time = time.time()
            if current_time - last_report >= 5:
                current_total = self.stats['total_operations']
                ops_per_sec = (current_total - last_total) / (current_time - last_report)
                elapsed = current_time - start_time
                
                print(f"  Progress: {elapsed:.0f}s elapsed, {current_total:,} ops, {ops_per_sec:.1f} ops/sec")
                
                last_report = current_time
                last_total = current_total
            
            time.sleep(1)
    
    def update_stats(self, result):
        """Update statistics with operation result"""
        self.stats['total_operations'] += 1
        
        if result['success']:
            self.stats['successful_operations'] += 1
            if result['operation'] == 'write':
                self.stats['total_bytes_written'] += result['data_size']
            else:
                self.stats['total_bytes_read'] += result['data_size']
        else:
            self.stats['failed_operations'] += 1
            if result['error']:
                self.stats['errors'].append(result['error'])
        
        # Update latency stats
        latency_ms = result['latency'] * 1000
        self.stats['min_latency'] = min(self.stats['min_latency'], latency_ms)
        self.stats['max_latency'] = max(self.stats['max_latency'], latency_ms)
        self.stats['latency_sum'] += latency_ms
    
    def run_stress_test(self, write_workers=5, read_workers=3, duration_seconds=60, 
                       write_ops_per_sec=8, read_ops_per_sec=15):
        """Run comprehensive stress test"""
        print(f"🔥 Starting Stress Test")
        print(f"   Duration: {duration_seconds}s")
        print(f"   Write workers: {write_workers} ({write_ops_per_sec} ops/sec each)")
        print(f"   Read workers: {read_workers} ({read_ops_per_sec} ops/sec each)")
        print(f"   Server: {self.server_url}")
        print()
        
        # Check server health
        try:
            response = requests.get(f"{self.server_url}/health_check", timeout=5)
            print("✅ Server health check passed")
        except:
            print("❌ Server not responding")
            return
        
        # Prepare some keys for read tests
        print("📝 Preparing test data...")
        read_keys = []
        for i in range(200):
            key = f"stress_prep_{i}_{int(time.time() * 1000000)}"
            data = self.generate_random_data(5000, 20000)  # 5KB to 20KB
            try:
                response = requests.put(
                    f"{self.server_url}/{key}",
                    data=data,
                    headers={"Content-Type": "application/octet-stream"},
                    timeout=10
                )
                if response.status_code == 200:
                    read_keys.append(key)
            except:
                pass
        
        print(f"✅ Prepared {len(read_keys)} keys for read testing")
        print()
        
        # Setup signal handler for graceful shutdown
        def signal_handler(signum, frame):
            print("\n⚠️  Stopping stress test...")
            self.stop_event.set()
        
        signal.signal(signal.SIGINT, signal_handler)
        
        # Start stress test
        self.stats['start_time'] = time.time()
        
        with ThreadPoolExecutor(max_workers=write_workers + read_workers + 1) as executor:
            futures = []
            
            # Start write workers
            for i in range(write_workers):
                future = executor.submit(
                    self.stress_write_worker, i, duration_seconds, write_ops_per_sec
                )
                futures.append(future)
            
            # Start read workers
            for i in range(read_workers):
                future = executor.submit(
                    self.stress_read_worker, i, read_keys, duration_seconds, read_ops_per_sec
                )
                futures.append(future)
            
            # Start monitor
            monitor_future = executor.submit(self.monitor_progress, duration_seconds)
            futures.append(monitor_future)
            
            print("🏃 Stress test running...")
            
            # Wait for completion
            for future in as_completed(futures):
                try:
                    future.result()
                except Exception as e:
                    print(f"Worker error: {e}")
        
        # Process any remaining results
        try:
            while True:
                result = self.results_queue.get_nowait()
                self.update_stats(result)
        except queue.Empty:
            pass
        
        # Display results
        self.display_results()
    
    def display_results(self):
        """Display stress test results"""
        print("\n" + "="*60)
        print("📊 STRESS TEST RESULTS")
        print("="*60)
        
        duration = time.time() - self.stats['start_time']
        
        # Main statistics table
        main_stats = [
            ["Test Duration", f"{duration:.1f} seconds"],
            ["Total Operations", f"{self.stats['total_operations']:,}"],
            ["Successful Operations", f"{self.stats['successful_operations']:,}"],
            ["Failed Operations", f"{self.stats['failed_operations']:,}"],
            ["Success Rate", f"{(self.stats['successful_operations']/max(self.stats['total_operations'],1)*100):.1f}%"],
            ["Overall Ops/Second", f"{self.stats['total_operations']/duration:.1f}"],
        ]
        
        print("\n📈 OVERALL PERFORMANCE")
        print(tabulate(main_stats, headers=["Metric", "Value"], tablefmt="grid"))
        
        # Throughput statistics
        throughput_stats = [
            ["Data Written", f"{self.stats['total_bytes_written']/1024/1024:.1f} MB"],
            ["Data Read", f"{self.stats['total_bytes_read']/1024/1024:.1f} MB"],
            ["Write Throughput", f"{self.stats['total_bytes_written']/duration/1024/1024:.2f} MB/s"],
            ["Read Throughput", f"{self.stats['total_bytes_read']/duration/1024/1024:.2f} MB/s"],
        ]
        
        print("\n🚀 THROUGHPUT")
        print(tabulate(throughput_stats, headers=["Metric", "Value"], tablefmt="grid"))
        
        # Latency statistics
        if self.stats['total_operations'] > 0:
            avg_latency = self.stats['latency_sum'] / self.stats['total_operations']
            latency_stats = [
                ["Min Latency", f"{self.stats['min_latency']:.1f} ms"],
                ["Max Latency", f"{self.stats['max_latency']:.1f} ms"],
                ["Average Latency", f"{avg_latency:.1f} ms"],
            ]
            
            print("\n⏱️  LATENCY")
            print(tabulate(latency_stats, headers=["Metric", "Value"], tablefmt="grid"))
        
        # Error summary
        if self.stats['errors']:
            error_counts = {}
            for error in self.stats['errors'][:100]:  # Limit to first 100 errors
                error_counts[error] = error_counts.get(error, 0) + 1
            
            print("\n❌ ERROR SUMMARY")
            error_table = [[error, count] for error, count in error_counts.items()]
            print(tabulate(error_table, headers=["Error", "Count"], tablefmt="grid"))
        
        print(f"\n✅ Stress test completed!")


def main():
    server_url = "http://localhost:8080"
    if len(sys.argv) > 1:
        server_url = sys.argv[1]
    
    duration = 30  # Default 30 seconds
    if len(sys.argv) > 2:
        duration = int(sys.argv[2])
    
    runner = StressTestRunner(server_url)
    
    try:
        runner.run_stress_test(
            write_workers=4,
            read_workers=6,
            duration_seconds=duration,
            write_ops_per_sec=5,  # Conservative to avoid overwhelming
            read_ops_per_sec=10
        )
    except KeyboardInterrupt:
        print("\n⚠️  Test interrupted by user")
        if runner.stats['total_operations'] > 0:
            runner.display_results()


if __name__ == "__main__":
    main()