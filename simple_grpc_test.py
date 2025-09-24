#!/usr/bin/env python3
"""
Simple gRPC endpoint latency test
Tests basic connectivity and response times to your gRPC endpoint
"""

import requests
import time
import json
from statistics import mean, median

def test_grpc_latency(endpoint, token, test_rounds=10):
    """Test basic gRPC endpoint latency"""
    headers = {
        "x-token": token,
        "Content-Type": "application/json"
    }

    latencies = []
    errors = 0

    print(f"🧪 Testing gRPC endpoint: {endpoint}")
    print(f"🔄 Running {test_rounds} rounds...")

    for round_num in range(1, test_rounds + 1):
        # Simple health check request
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

        except requests.exceptions.Timeout:
            print(f"  ⏰ Round {round_num}: Timeout")
            errors += 1
        except Exception as e:
            print(f"  ❌ Round {round_num}: Error - {e}")
            errors += 1

        # Brief pause
        time.sleep(0.5)

    return latencies, errors

def test_connection_only(endpoint, token, test_rounds=10):
    """Test just TCP connection latency (no request body)"""
    print(f"\n🔌 Testing connection latency to: {endpoint}")

    latencies = []
    errors = 0

    for round_num in range(1, test_rounds + 1):
        start_time = time.time()

        try:
            # Just test connectivity with minimal request
            response = requests.get(endpoint, timeout=5)
            end_time = time.time()
            latency_ms = (end_time - start_time) * 1000

            latencies.append(latency_ms)
            print(f"  🔗 Round {round_num}: {latency_ms:.2f}ms (HTTP {response.status_code})")

        except Exception as e:
            print(f"  ❌ Round {round_num}: {e}")
            errors += 1

        time.sleep(0.2)

    return latencies, errors

if __name__ == "__main__":
    # Your gRPC endpoint configuration
    endpoint = "https://grpc-ny6-1.erpc.global"
    token = "507c3fff-6dc7-4d6d-8915-596be560814f"

    print("⚔️  Simple gRPC Latency Test")
    print("🎯 Testing your gRPC endpoint for MEV suitability")
    print("=" * 60)

    # Test 1: Basic connectivity
    conn_latencies, conn_errors = test_connection_only(endpoint, token, 10)

    # Test 2: JSON-RPC requests
    grpc_latencies, grpc_errors = test_grpc_latency(endpoint, token, 10)

    print("\n" + "=" * 60)
    print("📊 RESULTS SUMMARY")
    print("=" * 60)

    if conn_latencies:
        print(f"🔗 Connection Latency:")
        print(f"   Average: {mean(conn_latencies):.2f}ms")
        print(f"   Median: {median(conn_latencies):.2f}ms")
        print(f"   Range: {min(conn_latencies):.2f}ms - {max(conn_latencies):.2f}ms")
        print(f"   Errors: {conn_errors}/10")

    if grpc_latencies:
        print(f"\n📡 gRPC Request Latency:")
        print(f"   Average: {mean(grpc_latencies):.2f}ms")
        print(f"   Median: {median(grpc_latencies):.2f}ms")
        print(f"   Range: {min(grpc_latencies):.2f}ms - {max(grpc_latencies):.2f}ms")
        print(f"   Errors: {grpc_errors}/10")

        # Compare with ShredStream baseline (from our test)
        shredstream_avg = 71.90  # From the latency test above

        grpc_avg = mean(grpc_latencies)
        print(f"\n⚔️  COMPARISON WITH SHREDSTREAM:")
        print(f"   ShredStream: {shredstream_avg:.2f}ms")
        print(f"   Your gRPC: {grpc_avg:.2f}ms")

        if grpc_avg < shredstream_avg:
            diff_pct = ((shredstream_avg - grpc_avg) / shredstream_avg) * 100
            print(f"   🏆 YOUR gRPC WINS: {diff_pct:.1}% faster!")
            print(f"   💡 RECOMMENDATION: Use gRPC as PRIMARY, ShredStream as backup")
        else:
            diff_pct = ((grpc_avg - shredstream_avg) / shredstream_avg) * 100
            if diff_pct < 25:  # Within 25% - Grok's "close to as fast" guidance
                print(f"   ⚖️  CLOSE PERFORMANCE: gRPC {diff_pct:.1}% slower")
                print(f"   💡 GROK GUIDANCE: Can use gRPC as primary (within acceptable range)")
            else:
                print(f"   📈 ShredStream faster: {diff_pct:.1}% difference")
                print(f"   💡 RECOMMENDATION: ShredStream primary, gRPC backup")
    else:
        print("\n❌ No successful gRPC requests - endpoint may need configuration")
        print("💡 RECOMMENDATION: Use ShredStream as primary data source")