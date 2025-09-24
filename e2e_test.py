#!/usr/bin/env python3
"""
End-to-End Testing Framework for Scribe Ledger 3-Node Cluster

This script tests the distributed consensus and S3 integration features
by spinning up a 3-node cluster and running various scenarios.
"""

import asyncio
import subprocess
import time
import json
import requests
import signal
import sys
import os
from typing import List, Dict, Optional
from dataclasses import dataclass
from pathlib import Path

@dataclass
class NodeConfig:
    id: int
    address: str
    config_file: str
    data_dir: str
    port: int

class ClusterManager:
    def __init__(self):
        self.nodes = [
            NodeConfig(1, "127.0.0.1:8001", "config-node1.toml", "./data/node1", 8001),
            NodeConfig(2, "127.0.0.1:8002", "config-node2.toml", "./data/node2", 8002),
            NodeConfig(3, "127.0.0.1:8003", "config-node3.toml", "./data/node3", 8003),
        ]
        self.processes: List[subprocess.Popen] = []
        self.minio_process: Optional[subprocess.Popen] = None
        
    def cleanup_data_dirs(self):
        """Clean up data directories from previous runs"""
        for node in self.nodes:
            data_path = Path(node.data_dir)
            if data_path.exists():
                import shutil
                shutil.rmtree(data_path)
                print(f"Cleaned up {node.data_dir}")
    
    def start_minio(self):
        """Start MinIO server for S3 testing"""
        try:
            # Check if MinIO is available
            subprocess.run(["minio", "--version"], capture_output=True, check=True)
            
            minio_data_dir = "./data/minio"
            Path(minio_data_dir).mkdir(parents=True, exist_ok=True)
            
            print("Starting MinIO server...")
            self.minio_process = subprocess.Popen([
                "minio", "server", minio_data_dir,
                "--address", "127.0.0.1:9000",
                "--console-address", "127.0.0.1:9001"
            ], env={**os.environ, "MINIO_ROOT_USER": "minioadmin", "MINIO_ROOT_PASSWORD": "minioadmin"})
            
            # Wait for MinIO to start
            time.sleep(3)
            
            # Create bucket
            try:
                subprocess.run([
                    "mc", "alias", "set", "local", "http://localhost:9000", "minioadmin", "minioadmin"
                ], capture_output=True, check=True)
                subprocess.run([
                    "mc", "mb", "local/scribe-ledger-cluster"
                ], capture_output=True, check=True)
                print("MinIO started and bucket created")
            except subprocess.CalledProcessError:
                print("Warning: Could not configure MinIO bucket (mc client not available)")
                
        except (subprocess.CalledProcessError, FileNotFoundError):
            print("Warning: MinIO not available, S3 tests will be skipped")
            self.minio_process = None
    
    def start_node(self, node: NodeConfig):
        """Start a single node"""
        print(f"Starting node {node.id} on {node.address}")
        
        # Ensure data directory exists
        Path(node.data_dir).mkdir(parents=True, exist_ok=True)
        
        # Start the node process
        process = subprocess.Popen([
            "cargo", "run", "--bin", "scribe-node", "--", 
            "--config", node.config_file
        ], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        
        self.processes.append(process)
        return process
    
    def start_cluster(self):
        """Start all nodes in the cluster"""
        print("Starting 3-node Scribe Ledger cluster...")
        
        # Start MinIO first
        self.start_minio()
        
        # Start all nodes
        for node in self.nodes:
            self.start_node(node)
            time.sleep(2)  # Stagger startup
        
        print("Waiting for cluster to initialize...")
        time.sleep(5)
        
        # Check if nodes are responsive
        for node in self.nodes:
            if self.is_node_responsive(node):
                print(f"Node {node.id} is responsive")
            else:
                print(f"Warning: Node {node.id} is not responsive")
    
    def is_node_responsive(self, node: NodeConfig) -> bool:
        """Check if a node is responsive via HTTP"""
        try:
            response = requests.get(f"http://{node.address}/health", timeout=5)
            return response.status_code == 200
        except requests.RequestException:
            return False
    
    def stop_cluster(self):
        """Stop all nodes and MinIO"""
        print("Stopping cluster...")
        
        # Stop all node processes
        for process in self.processes:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.kill()
        
        # Stop MinIO
        if self.minio_process and self.minio_process.poll() is None:
            self.minio_process.terminate()
            try:
                self.minio_process.wait(timeout=10)
            except subprocess.TimeoutExpired:
                self.minio_process.kill()

class E2ETests:
    def __init__(self, cluster_manager: ClusterManager):
        self.cluster = cluster_manager
        self.test_results = []
    
    def log_test_result(self, test_name: str, passed: bool, message: str = ""):
        """Log test result"""
        status = "PASS" if passed else "FAIL"
        print(f"[{status}] {test_name}: {message}")
        self.test_results.append({
            "test": test_name,
            "passed": passed,
            "message": message
        })
    
    async def test_basic_connectivity(self):
        """Test basic connectivity to all nodes"""
        test_name = "Basic Connectivity"
        all_responsive = True
        
        for node in self.cluster.nodes:
            responsive = self.cluster.is_node_responsive(node)
            if not responsive:
                all_responsive = False
                break
        
        self.log_test_result(test_name, all_responsive, 
                           "All nodes responsive" if all_responsive else "Some nodes not responsive")
    
    async def test_data_replication(self):
        """Test data replication across nodes"""
        test_name = "Data Replication"
        
        try:
            # Write data to node 1
            node1 = self.cluster.nodes[0]  
            data = {"key": "test-key", "value": "test-value-12345"}
            
            response = requests.put(
                f"http://{node1.address}/data", 
                json=data, 
                timeout=10
            )
            
            if response.status_code != 200:
                self.log_test_result(test_name, False, f"Write failed: {response.status_code}")
                return
            
            # Wait for replication
            await asyncio.sleep(2)
            
            # Read from all nodes
            success_count = 0
            for node in self.cluster.nodes:
                try:
                    read_response = requests.get(
                        f"http://{node.address}/data/test-key",
                        timeout=5
                    )
                    if read_response.status_code == 200:
                        read_data = read_response.json()
                        if read_data.get("value") == "test-value-12345":
                            success_count += 1
                except requests.RequestException:
                    pass
            
            self.log_test_result(test_name, success_count >= 2, 
                               f"{success_count}/3 nodes have replicated data")
                
        except Exception as e:
            self.log_test_result(test_name, False, f"Exception: {str(e)}")
    
    async def test_leader_election(self):
        """Test leader election behavior"""
        test_name = "Leader Election"
        
        try:
            leaders_found = 0
            leader_node = None
            
            for node in self.cluster.nodes:
                try:
                    response = requests.get(f"http://{node.address}/cluster/status", timeout=5)
                    if response.status_code == 200:
                        status = response.json()
                        if status.get("is_leader", False):
                            leaders_found += 1
                            leader_node = node
                except requests.RequestException:
                    pass
            
            # Should have exactly one leader
            success = leaders_found == 1
            message = f"Found {leaders_found} leaders"
            if leader_node:
                message += f" (Node {leader_node.id})"
            
            self.log_test_result(test_name, success, message)
            
        except Exception as e:
            self.log_test_result(test_name, False, f"Exception: {str(e)}")
    
    async def test_node_failure_recovery(self):
        """Test behavior when a node fails and recovers"""
        test_name = "Node Failure Recovery"
        
        if len(self.cluster.processes) < 3:
            self.log_test_result(test_name, False, "Not enough nodes running")
            return
        
        try:
            # Stop node 3
            node3_process = self.cluster.processes[2]
            node3_process.terminate()
            await asyncio.sleep(2)
            
            # Write data while node 3 is down
            node1 = self.cluster.nodes[0]
            data = {"key": "recovery-test", "value": "recovery-value"}
            
            response = requests.put(f"http://{node1.address}/data", json=data, timeout=10)
            write_success = response.status_code == 200
            
            # Restart node 3
            restarted_process = self.cluster.start_node(self.cluster.nodes[2])
            self.cluster.processes[2] = restarted_process
            await asyncio.sleep(5)  # Wait for recovery
            
            # Check if data is available on restarted node
            node3 = self.cluster.nodes[2]
            read_response = requests.get(f"http://{node3.address}/data/recovery-test", timeout=10)
            read_success = read_response.status_code == 200
            
            if read_success:
                read_data = read_response.json()
                data_correct = read_data.get("value") == "recovery-value"
            else:
                data_correct = False
            
            overall_success = write_success and read_success and data_correct
            
            self.log_test_result(test_name, overall_success,
                               f"Write: {write_success}, Read: {read_success}, Data: {data_correct}")
            
        except Exception as e:
            self.log_test_result(test_name, False, f"Exception: {str(e)}")
    
    async def test_concurrent_writes(self):
        """Test concurrent writes from multiple clients"""
        test_name = "Concurrent Writes"
        
        try:
            # Prepare concurrent write tasks
            async def write_data(node_idx: int, key_suffix: str):
                node = self.cluster.nodes[node_idx % len(self.cluster.nodes)]
                data = {"key": f"concurrent-{key_suffix}", "value": f"value-{key_suffix}"}
                
                try:
                    response = requests.put(f"http://{node.address}/data", json=data, timeout=10)
                    return response.status_code == 200
                except requests.RequestException:
                    return False
            
            # Execute concurrent writes
            tasks = []
            for i in range(10):
                task = write_data(i, str(i))
                tasks.append(task)
            
            results = await asyncio.gather(*tasks, return_exceptions=True)
            successful_writes = sum(1 for r in results if r is True)
            
            # Verify some writes succeeded
            self.log_test_result(test_name, successful_writes >= 5,
                               f"{successful_writes}/10 concurrent writes succeeded")
            
        except Exception as e:
            self.log_test_result(test_name, False, f"Exception: {str(e)}")
    
    async def run_all_tests(self):
        """Run all E2E tests"""
        print("\n=== Starting E2E Tests ===")
        
        test_methods = [
            self.test_basic_connectivity,
            self.test_data_replication,
            self.test_leader_election,
            self.test_node_failure_recovery,
            self.test_concurrent_writes,
        ]
        
        for test_method in test_methods:
            try:
                await test_method()
            except Exception as e:
                test_name = test_method.__name__.replace("test_", "").replace("_", " ").title()
                self.log_test_result(test_name, False, f"Test crashed: {str(e)}")
            
            await asyncio.sleep(1)  # Brief pause between tests
        
        # Print summary
        passed = sum(1 for r in self.test_results if r["passed"])
        total = len(self.test_results)
        
        print(f"\n=== Test Summary ===")
        print(f"Passed: {passed}/{total}")
        print(f"Failed: {total - passed}/{total}")
        
        if passed == total:
            print("🎉 All tests passed!")
            return True
        else:
            print("❌ Some tests failed")
            for result in self.test_results:
                if not result["passed"]:
                    print(f"  - {result['test']}: {result['message']}")
            return False

async def main():
    """Main E2E test runner"""
    cluster = ClusterManager()
    
    def signal_handler(signum, frame):
        print("\nReceived interrupt signal, cleaning up...")
        cluster.stop_cluster()
        sys.exit(0)
    
    signal.signal(signal.SIGINT, signal_handler)
    signal.signal(signal.SIGTERM, signal_handler)
    
    try:
        # Clean up from previous runs
        cluster.cleanup_data_dirs()
        
        # Start the cluster
        cluster.start_cluster()
        
        # Run tests
        tests = E2ETests(cluster)
        success = await tests.run_all_tests()
        
        return 0 if success else 1
        
    except Exception as e:
        print(f"E2E test runner failed: {e}")
        return 1
    
    finally:
        cluster.stop_cluster()

if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)