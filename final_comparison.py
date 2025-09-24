#!/usr/bin/env python3
"""
Final Comprehensive Comparison: Your gRPC vs Your ShredStream
Tests both your actual endpoints for MEV trading suitability
"""

import requests
import time
import asyncio
import aiohttp
import json
from statistics import mean, median
from concurrent.futures import ThreadPoolExecutor

def test_grpc_endpoint():
    """Test your gRPC endpoint latency"""
    endpoint = "https://grpc-ny6-1.erpc.global"
    token = "507c3fff-6dc7-4d6d-8915-596be560814f"

    headers = {
        "x-token": token,
        "Content-Type": "application/json"
    }

    latencies = []
    errors = 0
    test_rounds = 15

    print(f"🧪 Testing YOUR gRPC: {endpoint}")

    for round_num in range(1, test_rounds + 1):
        payload = {
            "jsonrpc": "2.0",
            "id": round_num,
            "method": "getHealth",
            "params": {}
        }

        start_time = time.time()

        try:
            response = requests.post(
                endpoint,
                headers=headers,
                json=payload,
                timeout=10
            )

            end_time = time.time()
            latency_ms = (end_time - start_time) * 1000

            if response.status_code == 200:
                latencies.append(latency_ms)
                print(f"  ✅ Round {round_num}: {latency_ms:.2f}ms")
            else:
                print(f"  ⚠️  Round {round_num}: HTTP {response.status_code}")
                errors += 1

        except Exception as e:
            print(f"  ❌ Round {round_num}: {e}")
            errors += 1

        time.sleep(0.3)

    return latencies, errors

def test_shredstream_endpoint():
    """Test your ShredStream endpoint latency"""
    endpoint = "https://shreds-ny6-1.erpc.global"
    # Use same token as gRPC for consistency
    token = "507c3fff-6dc7-4d6d-8915-596be560814f"

    headers = {
        "x-token": token,
        "Content-Type": "application/json",
        "authorization": f"Bearer {token}"
    }

    latencies = []
    errors = 0
    test_rounds = 15

    print(f"\n🧪 Testing YOUR SHREDSTREAM: {endpoint}")

    for round_num in range(1, test_rounds + 1):
        # Test with various potential gRPC/HTTP methods
        payloads = [
            {"jsonrpc": "2.0", "id": round_num, "method": "getHealth", "params": {}},
            {"jsonrpc": "2.0", "id": round_num, "method": "ping", "params": {}},
            # Simple connectivity test
        ]

        for i, payload in enumerate(payloads):
            start_time = time.time()

            try:
                response = requests.post(
                    endpoint,
                    headers=headers,
                    json=payload,
                    timeout=10
                )

                end_time = time.time()
                latency_ms = (end_time - start_time) * 1000

                if response.status_code == 200:
                    latencies.append(latency_ms)
                    print(f"  ✅ Round {round_num}.{i+1}: {latency_ms:.2f}ms")
                    break  # Success, move to next round
                elif i == len(payloads) - 1:  # Last attempt
                    print(f"  ⚠️  Round {round_num}: HTTP {response.status_code}")
                    errors += 1

            except Exception as e:
                if i == len(payloads) - 1:  # Last attempt
                    print(f"  ❌ Round {round_num}: {e}")
                    errors += 1

        time.sleep(0.3)

    return latencies, errors

def test_raw_connectivity():
    """Test raw TCP connectivity to both endpoints"""
    endpoints = {
        "gRPC": "https://grpc-ny6-1.erpc.global",
        "ShredStream": "https://shreds-ny6-1.erpc.global"
    }

    print(f"\n🔌 Testing Raw Connectivity:")

    for name, endpoint in endpoints.items():
        latencies = []

        for i in range(5):
            start_time = time.time()
            try:
                response = requests.get(endpoint, timeout=5)
                end_time = time.time()
                latency_ms = (end_time - start_time) * 1000
                latencies.append(latency_ms)
            except:
                latencies.append(999.0)  # Error value

        avg_latency = mean([l for l in latencies if l < 999])
        print(f"  {name}: {avg_latency:.2f}ms avg connectivity")

async def test_concurrent_load():
    """Test both endpoints under concurrent load"""
    print(f"\n⚡ Testing Concurrent Load (10 simultaneous requests):")

    async def test_endpoint_async(session, endpoint, headers, payload):
        start_time = time.time()
        try:
            async with session.post(endpoint, headers=headers, json=payload) as response:
                await response.text()
                return (time.time() - start_time) * 1000
        except:
            return 999.0

    # Test both endpoints concurrently
    grpc_endpoint = "https://grpc-ny6-1.erpc.global"
    shreds_endpoint = "https://shreds-ny6-1.erpc.global"
    token = "507c3fff-6dc7-4d6d-8915-596be560814f"

    headers = {
        "x-token": token,
        "Content-Type": "application/json"
    }

    payload = {"jsonrpc": "2.0", "id": 1, "method": "getHealth", "params": {}}

    async with aiohttp.ClientSession() as session:
        # Test gRPC under load
        grpc_tasks = [test_endpoint_async(session, grpc_endpoint, headers, payload) for _ in range(10)]
        grpc_results = await asyncio.gather(*grpc_tasks)
        grpc_concurrent = [r for r in grpc_results if r < 999]

        # Test ShredStream under load
        shreds_tasks = [test_endpoint_async(session, shreds_endpoint, headers, payload) for _ in range(10)]
        shreds_results = await asyncio.gather(*shreds_tasks)
        shreds_concurrent = [r for r in shreds_results if r < 999]

    if grpc_concurrent:
        print(f"  gRPC Concurrent: {mean(grpc_concurrent):.2f}ms avg ({len(grpc_concurrent)}/10 success)")

    if shreds_concurrent:
        print(f"  ShredStream Concurrent: {mean(shreds_concurrent):.2f}ms avg ({len(shreds_concurrent)}/10 success)")

def main():
    print("⚔️  FINAL COMPARISON: Your gRPC vs Your ShredStream")
    print("🎯 Testing YOUR actual endpoints for MEV trading")
    print("=" * 70)

    # Test raw connectivity first
    test_raw_connectivity()

    # Test your gRPC
    grpc_latencies, grpc_errors = test_grpc_endpoint()

    # Test your ShredStream
    shreds_latencies, shreds_errors = test_shredstream_endpoint()

    # Test concurrent load
    asyncio.run(test_concurrent_load())

    print("\n" + "=" * 70)
    print("📊 FINAL RESULTS")
    print("=" * 70)

    if grpc_latencies:
        grpc_avg = mean(grpc_latencies)
        grpc_med = median(grpc_latencies)
        print(f"📡 YOUR gRPC Performance:")
        print(f"   Average: {grpc_avg:.2f}ms")
        print(f"   Median: {grpc_med:.2f}ms")
        print(f"   Range: {min(grpc_latencies):.2f}ms - {max(grpc_latencies):.2f}ms")
        print(f"   Success Rate: {len(grpc_latencies)}/{len(grpc_latencies) + grpc_errors}")

    if shreds_latencies:
        shreds_avg = mean(shreds_latencies)
        shreds_med = median(shreds_latencies)
        print(f"\n🔗 YOUR SHREDSTREAM Performance:")
        print(f"   Average: {shreds_avg:.2f}ms")
        print(f"   Median: {shreds_med:.2f}ms")
        print(f"   Range: {min(shreds_latencies):.2f}ms - {max(shreds_latencies):.2f}ms")
        print(f"   Success Rate: {len(shreds_latencies)}/{len(shreds_latencies) + shreds_errors}")

    print(f"\n⚔️  HEAD-TO-HEAD COMPARISON:")
    if grpc_latencies and shreds_latencies:
        grpc_avg = mean(grpc_latencies)
        shreds_avg = mean(shreds_latencies)

        if grpc_avg < shreds_avg:
            diff_pct = ((shreds_avg - grpc_avg) / grpc_avg) * 100
            print(f"   🏆 WINNER: Your gRPC")
            print(f"   📊 Performance Gap: {diff_pct:.1}% faster ({grpc_avg:.2f}ms vs {shreds_avg:.2f}ms)")

            if diff_pct > 25:
                print(f"   💡 RECOMMENDATION: Use gRPC as PRIMARY (significant advantage)")
            else:
                print(f"   💡 GROK GUIDANCE: Both are competitive - gRPC has edge")

        else:
            diff_pct = ((grpc_avg - shreds_avg) / shreds_avg) * 100
            print(f"   🏆 WINNER: Your ShredStream")
            print(f"   📊 Performance Gap: {diff_pct:.1}% faster ({shreds_avg:.2f}ms vs {grpc_avg:.2f}ms)")

            if diff_pct > 25:
                print(f"   💡 RECOMMENDATION: Use ShredStream as PRIMARY")
            else:
                print(f"   💡 GROK GUIDANCE: Both are competitive - ShredStream has edge")

        # Performance tier analysis
        for name, avg in [("gRPC", grpc_avg), ("ShredStream", shreds_avg)]:
            if avg < 30:
                tier = "🔥 ELITE"
            elif avg < 50:
                tier = "🎯 COMPETITIVE"
            elif avg < 100:
                tier = "📊 GOOD"
            else:
                tier = "⚠️  NEEDS OPTIMIZATION"
            print(f"   {name} Tier: {tier} ({avg:.2f}ms)")

    elif grpc_latencies:
        print(f"   ✅ Only gRPC working - use as PRIMARY")
    elif shreds_latencies:
        print(f"   ✅ Only ShredStream working - use as PRIMARY")
    else:
        print(f"   ❌ Both endpoints need configuration")

    print(f"\n🎯 FINAL RECOMMENDATION:")
    if grpc_latencies and shreds_latencies:
        grpc_avg = mean(grpc_latencies)
        shreds_avg = mean(shreds_latencies)
        faster = "gRPC" if grpc_avg < shreds_avg else "ShredStream"
        slower = "ShredStream" if grpc_avg < shreds_avg else "gRPC"

        print(f"   PRIMARY: Your {faster} (faster)")
        print(f"   BACKUP: Your {slower} (redundancy)")
        print(f"   📈 Both endpoints are YOUR infrastructure - excellent setup!")

    print(f"\n✨ Both systems appear to be on the same infrastructure (erpc.global)")
    print(f"   This gives you excellent redundancy and flexibility! 🚀")

if __name__ == "__main__":
    main()