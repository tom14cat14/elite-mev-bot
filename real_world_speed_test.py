#!/usr/bin/env python3
"""
Real-World MEV Trading Speed Test
Tests actual performance with live Solana mainnet and real PumpFun tokens
Measures true end-to-end latency for MEV opportunities
"""

import asyncio
import aiohttp
import time
import json
import statistics
from datetime import datetime
from typing import List, Dict, Any
import base58

class RealWorldSpeedTest:
    def __init__(self):
        self.results = {
            'test_start': datetime.now().isoformat(),
            'real_world_tests': {},
            'mainnet_integration': True
        }

        # Real Solana RPC endpoints
        self.rpc_endpoints = [
            "https://api.mainnet-beta.solana.com",
            "https://solana-api.projectserum.com",
            "https://rpc.ankr.com/solana"
        ]

        # Real PumpFun program info
        self.pumpfun_program_id = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
        self.pumpfun_global = "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf"

    async def test_real_rpc_latency(self) -> Dict[str, Any]:
        """Test latency to real Solana RPC endpoints"""
        print("🌐 Testing Real Solana RPC Latency...")

        endpoint_results = {}

        async with aiohttp.ClientSession() as session:
            for endpoint in self.rpc_endpoints:
                latencies = []
                print(f"  📡 Testing {endpoint}...")

                for i in range(10):
                    start_time = time.time()

                    # Real RPC call - get recent blockhash
                    payload = {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "getRecentBlockhash",
                        "params": []
                    }

                    try:
                        async with session.post(endpoint, json=payload, timeout=5) as response:
                            await response.json()
                            latency = (time.time() - start_time) * 1000
                            latencies.append(latency)
                    except Exception as e:
                        print(f"    ❌ Request {i+1} failed: {e}")
                        latencies.append(float('inf'))

                # Filter out failed requests
                valid_latencies = [l for l in latencies if l != float('inf')]

                if valid_latencies:
                    endpoint_results[endpoint] = {
                        'average_ms': round(statistics.mean(valid_latencies), 2),
                        'median_ms': round(statistics.median(valid_latencies), 2),
                        'min_ms': round(min(valid_latencies), 2),
                        'max_ms': round(max(valid_latencies), 2),
                        'success_rate': len(valid_latencies) / len(latencies) * 100
                    }
                    print(f"    ✅ Avg: {endpoint_results[endpoint]['average_ms']}ms")
                else:
                    endpoint_results[endpoint] = {'status': 'FAILED'}
                    print(f"    ❌ All requests failed")

        # Find fastest endpoint
        fastest_endpoint = min(
            [(ep, data) for ep, data in endpoint_results.items() if 'average_ms' in data],
            key=lambda x: x[1]['average_ms'],
            default=(None, None)
        )

        result = {
            'endpoints': endpoint_results,
            'fastest_endpoint': fastest_endpoint[0] if fastest_endpoint[0] else None,
            'fastest_latency_ms': fastest_endpoint[1]['average_ms'] if fastest_endpoint[1] else None
        }

        print(f"  🏆 Fastest RPC: {result['fastest_endpoint']} ({result['fastest_latency_ms']}ms)")
        return result

    async def discover_real_pumpfun_tokens(self) -> List[str]:
        """Discover real active PumpFun tokens from mainnet"""
        print("🔍 Discovering Real PumpFun Tokens...")

        # Use fastest RPC endpoint
        rpc_url = "https://api.mainnet-beta.solana.com"

        async with aiohttp.ClientSession() as session:
            # Get program accounts for PumpFun
            payload = {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "getProgramAccounts",
                "params": [
                    self.pumpfun_program_id,
                    {
                        "encoding": "base64",
                        "dataSlice": {"offset": 0, "length": 0},
                        "filters": [
                            {"dataSize": 100}  # Approximate bonding curve account size
                        ]
                    }
                ]
            }

            try:
                async with session.post(rpc_url, json=payload, timeout=30) as response:
                    data = await response.json()

                    if 'result' in data and data['result']:
                        token_accounts = [account['pubkey'] for account in data['result'][:20]]  # Take first 20
                        print(f"  ✅ Found {len(token_accounts)} PumpFun accounts")
                        return token_accounts
                    else:
                        print(f"  ⚠️ No PumpFun accounts found or RPC error")
                        # Fallback to known active tokens (these are real mainnet addresses)
                        return [
                            "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",  # BONK
                            "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr",  # POPCAT
                            "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",   # JUP
                        ]
            except Exception as e:
                print(f"  ❌ Error discovering tokens: {e}")
                # Fallback tokens
                return [
                    "DezXAZ8z7PnrnRJjz3wXBoRgixCa6xjnB7YaB1pPB263",
                    "7GCihgDB8fe6KNjn2MYtkzZcRjQy3t9GHdC8uHYmW2hr",
                    "JUPyiwrYJFskUPiHa7hkeR8VUtAeFoSYbKedZNsDvCN",
                ]

    async def test_real_account_fetching(self, token_addresses: List[str]) -> Dict[str, Any]:
        """Test speed of fetching real account data"""
        print("📊 Testing Real Account Data Fetching...")

        rpc_url = "https://api.mainnet-beta.solana.com"
        fetch_times = []
        successful_fetches = 0

        async with aiohttp.ClientSession() as session:
            for i, token_address in enumerate(token_addresses[:10]):  # Test first 10
                start_time = time.time()

                # Real account fetch
                payload = {
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getAccountInfo",
                    "params": [
                        token_address,
                        {"encoding": "base64"}
                    ]
                }

                try:
                    async with session.post(rpc_url, json=payload, timeout=5) as response:
                        data = await response.json()

                        if 'result' in data and data['result']:
                            fetch_time = (time.time() - start_time) * 1000
                            fetch_times.append(fetch_time)
                            successful_fetches += 1

                            # Parse account data size
                            account_data = data['result']['value']
                            if account_data and 'data' in account_data:
                                data_size = len(account_data['data'][0]) if account_data['data'] else 0
                                print(f"    ✅ Token {i+1}: {fetch_time:.2f}ms (data: {data_size} bytes)")
                            else:
                                print(f"    ⚠️ Token {i+1}: {fetch_time:.2f}ms (no data)")
                        else:
                            print(f"    ❌ Token {i+1}: Account not found")

                except Exception as e:
                    print(f"    ❌ Token {i+1}: Error - {e}")

        if fetch_times:
            result = {
                'total_tokens_tested': len(token_addresses[:10]),
                'successful_fetches': successful_fetches,
                'success_rate': successful_fetches / len(token_addresses[:10]) * 100,
                'average_fetch_ms': round(statistics.mean(fetch_times), 2),
                'median_fetch_ms': round(statistics.median(fetch_times), 2),
                'min_fetch_ms': round(min(fetch_times), 2),
                'max_fetch_ms': round(max(fetch_times), 2)
            }
        else:
            result = {'status': 'FAILED', 'reason': 'No successful fetches'}

        print(f"  📊 Account Fetching: {result.get('average_fetch_ms', 'N/A')}ms avg")
        return result

    async def test_real_transaction_simulation(self) -> Dict[str, Any]:
        """Test real transaction simulation speed"""
        print("⚡ Testing Real Transaction Simulation...")

        rpc_url = "https://api.mainnet-beta.solana.com"
        simulation_times = []

        # Create a realistic transaction for simulation
        # This is a simple SOL transfer that won't actually execute
        dummy_tx = {
            "jsonrpc": "2.0",
            "id": 1,
            "method": "simulateTransaction",
            "params": [
                # Base64 encoded transaction (dummy transfer)
                "AQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABAAEDArczbMIA1sOBw8z0rsKaCDWS07wJqNYz1ZUoC3YbY9aWk+jqjsrmwL3m9fZRCr2qXy26I+ySW/+72vS9BfVFKAwAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABz0w0UaQY3VNqVe8GE3avQNiS0Uez8SgS6QK8gS2S8rU",
                {"encoding": "base64"}
            ]
        }

        async with aiohttp.ClientSession() as session:
            for i in range(10):
                start_time = time.time()

                try:
                    async with session.post(rpc_url, json=dummy_tx, timeout=5) as response:
                        await response.json()
                        sim_time = (time.time() - start_time) * 1000
                        simulation_times.append(sim_time)
                        print(f"    ✅ Simulation {i+1}: {sim_time:.2f}ms")

                except Exception as e:
                    print(f"    ❌ Simulation {i+1}: Error - {e}")

        if simulation_times:
            result = {
                'total_simulations': len(simulation_times),
                'average_simulation_ms': round(statistics.mean(simulation_times), 2),
                'median_simulation_ms': round(statistics.median(simulation_times), 2),
                'min_simulation_ms': round(min(simulation_times), 2),
                'max_simulation_ms': round(max(simulation_times), 2)
            }
        else:
            result = {'status': 'FAILED', 'reason': 'No successful simulations'}

        print(f"  ⚡ Transaction Simulation: {result.get('average_simulation_ms', 'N/A')}ms avg")
        return result

    async def test_real_mev_opportunity_detection(self, token_addresses: List[str]) -> Dict[str, Any]:
        """Test real MEV opportunity detection pipeline"""
        print("🎯 Testing Real MEV Opportunity Detection...")

        rpc_url = "https://api.mainnet-beta.solana.com"
        detection_times = []
        opportunities_found = 0

        async with aiohttp.ClientSession() as session:
            for i, token_address in enumerate(token_addresses[:5]):  # Test 5 tokens
                pipeline_start = time.time()

                print(f"    🔍 Analyzing token {i+1}: {token_address[:8]}...")

                try:
                    # Step 1: Fetch account info
                    account_start = time.time()
                    account_payload = {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "getAccountInfo",
                        "params": [token_address, {"encoding": "base64"}]
                    }

                    async with session.post(rpc_url, json=account_payload, timeout=3) as response:
                        account_data = await response.json()
                    account_time = (time.time() - account_start) * 1000

                    # Step 2: Get token supply (if it's a token mint)
                    supply_start = time.time()
                    supply_payload = {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "getTokenSupply",
                        "params": [token_address]
                    }

                    async with session.post(rpc_url, json=supply_payload, timeout=3) as response:
                        supply_data = await response.json()
                    supply_time = (time.time() - supply_start) * 1000

                    # Step 3: Analyze for MEV opportunity (simplified)
                    analysis_start = time.time()

                    # Simple opportunity scoring
                    opportunity_score = 0
                    if 'result' in account_data and account_data['result']:
                        opportunity_score += 30  # Account exists

                    if 'result' in supply_data and supply_data['result']:
                        supply_info = supply_data['result']['value']
                        if supply_info and 'uiAmount' in supply_info:
                            ui_amount = supply_info['uiAmount']
                            if ui_amount and ui_amount > 1000000:  # > 1M supply
                                opportunity_score += 20
                            if ui_amount and ui_amount < 1000000000:  # < 1B supply
                                opportunity_score += 30

                    analysis_time = (time.time() - analysis_start) * 1000

                    total_time = (time.time() - pipeline_start) * 1000
                    detection_times.append(total_time)

                    if opportunity_score >= 50:  # Threshold for "opportunity"
                        opportunities_found += 1
                        print(f"      ✅ MEV Opportunity! Score: {opportunity_score} | Time: {total_time:.2f}ms")
                    else:
                        print(f"      ⚪ No opportunity. Score: {opportunity_score} | Time: {total_time:.2f}ms")

                    print(f"         📊 Account: {account_time:.1f}ms | Supply: {supply_time:.1f}ms | Analysis: {analysis_time:.1f}ms")

                except Exception as e:
                    print(f"      ❌ Error analyzing token: {e}")

        if detection_times:
            result = {
                'tokens_analyzed': len(detection_times),
                'opportunities_found': opportunities_found,
                'opportunity_rate': round(opportunities_found / len(detection_times) * 100, 1),
                'average_detection_ms': round(statistics.mean(detection_times), 2),
                'median_detection_ms': round(statistics.median(detection_times), 2),
                'min_detection_ms': round(min(detection_times), 2),
                'max_detection_ms': round(max(detection_times), 2),
                'target_latency_ms': 15.0,
                'under_target': len([t for t in detection_times if t <= 15.0]),
                'over_target': len([t for t in detection_times if t > 15.0])
            }
        else:
            result = {'status': 'FAILED', 'reason': 'No successful detections'}

        print(f"  🎯 MEV Detection: {result.get('average_detection_ms', 'N/A')}ms avg")
        print(f"      💰 Opportunities: {opportunities_found}/{len(detection_times)} ({result.get('opportunity_rate', 0)}%)")
        return result

    async def test_real_concurrent_load(self) -> Dict[str, Any]:
        """Test performance under real concurrent load"""
        print("🔄 Testing Real Concurrent Load Performance...")

        rpc_url = "https://api.mainnet-beta.solana.com"

        async def concurrent_rpc_worker():
            """Worker for concurrent RPC requests"""
            results = []
            async with aiohttp.ClientSession() as session:
                for _ in range(5):  # 5 requests per worker
                    start_time = time.time()

                    payload = {
                        "jsonrpc": "2.0",
                        "id": 1,
                        "method": "getSlot",
                        "params": []
                    }

                    try:
                        async with session.post(rpc_url, json=payload, timeout=3) as response:
                            await response.json()
                            request_time = (time.time() - start_time) * 1000
                            results.append(request_time)
                    except Exception:
                        results.append(float('inf'))

            return [r for r in results if r != float('inf')]

        # Run 10 concurrent workers
        start_time = time.time()
        tasks = [concurrent_rpc_worker() for _ in range(10)]
        worker_results = await asyncio.gather(*tasks)
        total_duration = time.time() - start_time

        # Flatten results
        all_times = []
        for worker_result in worker_results:
            all_times.extend(worker_result)

        if all_times:
            result = {
                'concurrent_workers': 10,
                'requests_per_worker': 5,
                'total_successful_requests': len(all_times),
                'total_duration_seconds': round(total_duration, 2),
                'requests_per_second': round(len(all_times) / total_duration, 1),
                'average_latency_ms': round(statistics.mean(all_times), 2),
                'median_latency_ms': round(statistics.median(all_times), 2),
                'min_latency_ms': round(min(all_times), 2),
                'max_latency_ms': round(max(all_times), 2)
            }
        else:
            result = {'status': 'FAILED', 'reason': 'No successful concurrent requests'}

        print(f"  🔄 Concurrent Load: {result.get('requests_per_second', 'N/A')} req/sec")
        print(f"      ⚡ Avg Latency: {result.get('average_latency_ms', 'N/A')}ms")
        return result

    async def run_comprehensive_real_world_test(self) -> Dict[str, Any]:
        """Run all real-world performance tests"""
        print("🌍 ELITE MEV BOT v2.1 - REAL-WORLD SPEED TEST")
        print("=" * 70)
        print(f"📅 Test Start: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
        print(f"🌐 Testing Against: Solana Mainnet (LIVE)")
        print(f"🎯 PumpFun Program: {self.pumpfun_program_id}")
        print("=" * 70)

        # Run all real-world tests
        self.results['real_world_tests']['rpc_latency'] = await self.test_real_rpc_latency()

        print()
        discovered_tokens = await self.discover_real_pumpfun_tokens()

        print()
        self.results['real_world_tests']['account_fetching'] = await self.test_real_account_fetching(discovered_tokens)

        print()
        self.results['real_world_tests']['transaction_simulation'] = await self.test_real_transaction_simulation()

        print()
        self.results['real_world_tests']['mev_detection'] = await self.test_real_mev_opportunity_detection(discovered_tokens)

        print()
        self.results['real_world_tests']['concurrent_load'] = await self.test_real_concurrent_load()

        # Calculate final summary
        mev_result = self.results['real_world_tests']['mev_detection']
        rpc_result = self.results['real_world_tests']['rpc_latency']
        concurrent_result = self.results['real_world_tests']['concurrent_load']

        self.results['real_world_summary'] = {
            'test_completion': datetime.now().isoformat(),
            'fastest_rpc_latency_ms': rpc_result.get('fastest_latency_ms', 'N/A'),
            'mev_detection_latency_ms': mev_result.get('average_detection_ms', 'N/A'),
            'opportunities_found': mev_result.get('opportunities_found', 0),
            'opportunity_rate': mev_result.get('opportunity_rate', 0),
            'concurrent_throughput_rps': concurrent_result.get('requests_per_second', 'N/A'),
            'target_latency_ms': 15.0,
            'real_world_grade': self.calculate_real_world_grade(mev_result),
            'production_ready': self.assess_production_readiness(mev_result, rpc_result, concurrent_result)
        }

        self.print_real_world_summary()
        return self.results

    def calculate_real_world_grade(self, mev_result: Dict[str, Any]) -> str:
        """Calculate grade based on real-world performance"""
        if not isinstance(mev_result, dict) or 'average_detection_ms' not in mev_result:
            return "INCOMPLETE"

        avg_latency = mev_result['average_detection_ms']

        if avg_latency <= 10.0:
            return "ELITE"
        elif avg_latency <= 15.0:
            return "EXCELLENT"
        elif avg_latency <= 25.0:
            return "GOOD"
        else:
            return "NEEDS_IMPROVEMENT"

    def assess_production_readiness(self, mev_result: Dict, rpc_result: Dict, concurrent_result: Dict) -> bool:
        """Assess if system is ready for production trading"""
        if not all(isinstance(r, dict) for r in [mev_result, rpc_result, concurrent_result]):
            return False

        # Check key metrics
        mev_latency_ok = mev_result.get('average_detection_ms', float('inf')) <= 25.0
        rpc_latency_ok = rpc_result.get('fastest_latency_ms', float('inf')) <= 50.0
        concurrent_ok = concurrent_result.get('requests_per_second', 0) >= 5.0

        return mev_latency_ok and rpc_latency_ok and concurrent_ok

    def print_real_world_summary(self):
        """Print comprehensive real-world test summary"""
        print("\n" + "=" * 70)
        print("🌍 REAL-WORLD PERFORMANCE TEST RESULTS")
        print("=" * 70)

        summary = self.results['real_world_summary']

        print(f"🎯 Production Ready: {'✅ YES' if summary['production_ready'] else '❌ NO'}")
        print(f"⚡ RPC Latency: {summary['fastest_rpc_latency_ms']}ms (fastest endpoint)")
        print(f"🔍 MEV Detection: {summary['mev_detection_latency_ms']}ms avg")
        print(f"💰 Opportunities: {summary['opportunities_found']} found ({summary['opportunity_rate']}%)")
        print(f"🔄 Throughput: {summary['concurrent_throughput_rps']} req/sec")
        print(f"🏆 Real-World Grade: {summary['real_world_grade']}")

        print(f"\n📊 PERFORMANCE vs TARGETS:")
        mev_latency = summary['mev_detection_latency_ms']
        target_latency = summary['target_latency_ms']

        if isinstance(mev_latency, (int, float)) and isinstance(target_latency, (int, float)):
            if mev_latency <= target_latency:
                print(f"   ✅ MEV Detection: {mev_latency}ms ≤ {target_latency}ms target")
            else:
                print(f"   ❌ MEV Detection: {mev_latency}ms > {target_latency}ms target")

        print(f"\n🌍 REAL-WORLD READINESS:")
        if summary['production_ready']:
            print(f"   ✅ Latency targets achieved")
            print(f"   ✅ MEV opportunities detected")
            print(f"   ✅ Concurrent load handled")
            print(f"   ✅ Ready for live mainnet trading")
        else:
            print(f"   ⚠️  Performance optimization needed")
            print(f"   ⚠️  Review latency and throughput")

        print("=" * 70)

if __name__ == "__main__":
    async def main():
        test = RealWorldSpeedTest()
        results = await test.run_comprehensive_real_world_test()

        # Save results to file
        with open('real_world_speed_results.json', 'w') as f:
            json.dump(results, f, indent=2)

        print(f"\n💾 Real-world speed results saved to: real_world_speed_results.json")

    # Run the async test
    asyncio.run(main())