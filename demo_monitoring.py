#!/usr/bin/env python3
"""
Demo script for Scribe Ledger Real-time Raft Monitoring System
This script demonstrates the monitoring capabilities we've implemented:
1. REST API endpoints for status, metrics, and events
2. WebSocket real-time event streaming
3. Performance monitoring and statistics
"""

import asyncio
import websockets
import requests
import json
import time
from typing import Dict, Any

class ScribeLedgerMonitoringDemo:
    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url
        self.ws_url = base_url.replace("http://", "ws://") + "/raft/live"
        
    def print_banner(self):
        print("""
╦ ╦╦ ╦╦═╗╔═╗  ╔═╗╔═╗╦═╗╦╔╗ ╔═╗  ╦  ╔═╗╔╦╗╔═╗╔═╗╦═╗
╠═╣╚╦╝╠╦╝╠═╣  ╚═╗║  ╠╦╝║╠╩╗║╣   ║  ║╣  ║║║ ╦║╣ ╠╦╝
╩ ╩ ╩ ╩╚═╩ ╩  ╚═╝╚═╝╩╚═╩╚═╝╚═╝  ╩═╝╚═╝═╩╝╚═╝╚═╝╩╚═

🔗 Real-time Raft Monitoring System Demo
📊 Showcasing distributed consensus monitoring capabilities
        """)
    
    def test_raft_status(self) -> Dict[str, Any]:
        """Test the /raft/status endpoint"""
        print("\n📊 Testing Raft Status Endpoint...")
        try:
            response = requests.get(f"{self.base_url}/raft/status", timeout=5)
            if response.status_code == 200:
                status = response.json()
                print(f"✅ Status: {json.dumps(status, indent=2)}")
                return status
            elif response.status_code == 503:
                print("⚠️ Consensus node not available (service unavailable)")
                return {"error": "service_unavailable"}
            else:
                print(f"❌ Error: {response.status_code}")
                return {"error": f"http_{response.status_code}"}
        except requests.RequestException as e:
            print(f"❌ Connection failed: {e}")
            return {"error": "connection_failed"}
    
    def test_raft_metrics(self) -> Dict[str, Any]:
        """Test the /raft/metrics endpoint"""
        print("\n📈 Testing Raft Metrics Endpoint...")
        try:
            response = requests.get(f"{self.base_url}/raft/metrics", timeout=5)
            if response.status_code == 200:
                metrics = response.json()
                print(f"✅ Metrics: {json.dumps(metrics, indent=2)}")
                return metrics
            elif response.status_code == 503:
                print("⚠️ Consensus node not available (service unavailable)")
                return {"error": "service_unavailable"}
            else:
                print(f"❌ Error: {response.status_code}")
                return {"error": f"http_{response.status_code}"}
        except requests.RequestException as e:
            print(f"❌ Connection failed: {e}")
            return {"error": "connection_failed"}
    
    def test_raft_events(self) -> Dict[str, Any]:
        """Test the /raft/events endpoint"""
        print("\n📋 Testing Raft Events Endpoint...")
        try:
            response = requests.get(f"{self.base_url}/raft/events", timeout=5)
            if response.status_code == 200:
                events = response.json()
                print(f"✅ Recent Events (count: {events.get('count', 0)}):")
                for i, event in enumerate(events.get('events', [])[:3]):  # Show first 3 events
                    print(f"  {i+1}. [{event.get('severity', 'INFO')}] {event.get('event', {}).get('type', 'Unknown')}")
                return events
            elif response.status_code == 503:
                print("⚠️ Consensus node not available (service unavailable)")
                return {"error": "service_unavailable"}
            else:
                print(f"❌ Error: {response.status_code}")
                return {"error": f"http_{response.status_code}"}
        except requests.RequestException as e:
            print(f"❌ Connection failed: {e}")
            return {"error": "connection_failed"}
    
    async def test_websocket_streaming(self, duration: int = 10):
        """Test WebSocket real-time event streaming"""
        print(f"\n🔄 Testing WebSocket Real-time Streaming (for {duration}s)...")
        try:
            async with websockets.connect(self.ws_url) as websocket:
                print(f"✅ Connected to {self.ws_url}")
                
                start_time = time.time()
                event_count = 0
                
                while time.time() - start_time < duration:
                    try:
                        # Set a short timeout to check duration regularly
                        message = await asyncio.wait_for(websocket.recv(), timeout=1.0)
                        data = json.loads(message)
                        event_count += 1
                        
                        msg_type = data.get('type', 'unknown')
                        if msg_type == 'event':
                            event_data = data.get('data', {})
                            print(f"  📨 Event: {event_data.get('event', {}).get('type', 'Unknown')} "
                                  f"[{event_data.get('severity', 'INFO')}]")
                        elif msg_type == 'heartbeat':
                            print(f"  💗 Heartbeat: {data.get('timestamp')}")
                        elif msg_type == 'status':
                            print(f"  ℹ️ Status: {data.get('message')}")
                        else:
                            print(f"  📩 {msg_type}: {data}")
                    
                    except asyncio.TimeoutError:
                        # No message received, continue waiting
                        continue
                
                print(f"✅ Received {event_count} messages in {duration} seconds")
                return {"success": True, "message_count": event_count}
                
        except Exception as e:
            print(f"❌ WebSocket connection failed: {e}")
            return {"error": "websocket_failed", "message": str(e)}
    
    async def run_full_demo(self):
        """Run the complete monitoring demo"""
        self.print_banner()
        
        print("🚀 Starting Scribe Ledger Monitoring Demo...")
        print("ℹ️  Make sure the Scribe Ledger node is running with: cargo run --bin scribe-node")
        print("ℹ️  Default server should be at: http://localhost:8080")
        
        # Test REST endpoints
        status_result = self.test_raft_status()
        metrics_result = self.test_raft_metrics()
        events_result = self.test_raft_events()
        
        # Test WebSocket streaming
        ws_result = await self.test_websocket_streaming(duration=10)
        
        # Summary
        print("\n" + "="*60)
        print("📊 DEMO SUMMARY")
        print("="*60)
        
        endpoints_working = sum([
            1 for result in [status_result, metrics_result, events_result]
            if not result.get('error')
        ])
        
        print(f"✅ REST Endpoints Working: {endpoints_working}/3")
        print(f"✅ WebSocket Streaming: {'Working' if ws_result.get('success') else 'Failed'}")
        
        if all(not result.get('error') for result in [status_result, metrics_result, events_result]):
            print("🎉 All monitoring endpoints are functioning correctly!")
            print("\n📚 Available Endpoints:")
            print(f"   • GET  {self.base_url}/raft/status  - Current Raft node status")
            print(f"   • GET  {self.base_url}/raft/metrics - Performance metrics")
            print(f"   • GET  {self.base_url}/raft/events  - Recent Raft events")
            print(f"   • WS   {self.ws_url}        - Real-time event stream")
        else:
            print("⚠️  Some endpoints may not be available. This is expected if:")
            print("   • The server is running in local-only mode (no consensus)")
            print("   • The consensus node is not started")
            print("   • The server is not running")
        
        print(f"\n🏁 Demo completed at {time.strftime('%Y-%m-%d %H:%M:%S')}")

if __name__ == "__main__":
    demo = ScribeLedgerMonitoringDemo()
    asyncio.run(demo.run_full_demo())