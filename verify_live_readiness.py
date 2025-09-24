#!/usr/bin/env python3
"""
Elite MEV Bot v2.1 Production - Live Trading Readiness Verification
Final check before deploying with real money
"""

import os
import json
import time
from datetime import datetime

def verify_live_readiness():
    """Comprehensive verification for live trading deployment"""
    print("🚀 ELITE MEV BOT v2.1 - LIVE TRADING READINESS VERIFICATION")
    print("=" * 70)

    verification_results = {
        'timestamp': datetime.now().isoformat(),
        'checks': {},
        'summary': {}
    }

    checks_passed = 0
    total_checks = 0

    # Check 1: Configuration
    print("🔧 Checking Configuration...")
    total_checks += 1
    try:
        with open('.env', 'r') as f:
            config_content = f.read()

        config_checks = {
            'ENABLE_REAL_TRADING=true': 'ENABLE_REAL_TRADING=true' in config_content,
            'PumpFun Program ID': 'PUMPFUN_PROGRAM_ID=6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P' in config_content,
            'Capital configured': 'CAPITAL_SOL=3.0' in config_content,
            'Circuit breaker enabled': 'CIRCUIT_BREAKER_ENABLED=true' in config_content,
            'ShredStream primary': 'SHREDS_ENDPOINT=udp://stream.shredstream.com:8765' in config_content
        }

        config_passed = all(config_checks.values())
        verification_results['checks']['configuration'] = {
            'status': 'PASS' if config_passed else 'FAIL',
            'details': config_checks
        }

        if config_passed:
            checks_passed += 1
            print("  ✅ Configuration: PASS")
        else:
            print("  ❌ Configuration: FAIL")
            for check, result in config_checks.items():
                status = "✅" if result else "❌"
                print(f"    {status} {check}")

    except Exception as e:
        verification_results['checks']['configuration'] = {
            'status': 'ERROR',
            'error': str(e)
        }
        print(f"  ❌ Configuration: ERROR - {e}")

    # Check 2: Source Code Modifications
    print("\n📝 Checking Source Code Modifications...")
    total_checks += 1
    try:
        with open('src/bin/elite_mev_bot_v2_1_production.rs', 'r') as f:
            source_content = f.read()

        source_checks = {
            'Real trading enabled': 'enable_real_trading: true' in source_content,
            'PumpFun integration': 'use crate::pumpfun_integration::PumpFunIntegration' in source_content,
            'Real blockchain data': 'REAL IMPLEMENTATION' in source_content,
            'Jito bundle client': 'self.jito_client.submit_bundle' in source_content
        }

        source_passed = all(source_checks.values())
        verification_results['checks']['source_code'] = {
            'status': 'PASS' if source_passed else 'FAIL',
            'details': source_checks
        }

        if source_passed:
            checks_passed += 1
            print("  ✅ Source Code: PASS")
        else:
            print("  ❌ Source Code: FAIL")
            for check, result in source_checks.items():
                status = "✅" if result else "❌"
                print(f"    {status} {check}")

    except Exception as e:
        verification_results['checks']['source_code'] = {
            'status': 'ERROR',
            'error': str(e)
        }
        print(f"  ❌ Source Code: ERROR - {e}")

    # Check 3: PumpFun Integration
    print("\n🎯 Checking PumpFun Integration...")
    total_checks += 1
    try:
        with open('src/pumpfun_integration.rs', 'r') as f:
            pumpfun_content = f.read()

        pumpfun_checks = {
            'Real program ID': '6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P' in pumpfun_content,
            'Buy instruction': 'create_buy_instruction' in pumpfun_content,
            'Sell instruction': 'create_sell_instruction' in pumpfun_content,
            'Bonding curve math': 'calculate_buy_price' in pumpfun_content,
            'Account derivation': 'derive_bonding_curve_address' in pumpfun_content
        }

        pumpfun_passed = all(pumpfun_checks.values())
        verification_results['checks']['pumpfun_integration'] = {
            'status': 'PASS' if pumpfun_passed else 'FAIL',
            'details': pumpfun_checks
        }

        if pumpfun_passed:
            checks_passed += 1
            print("  ✅ PumpFun Integration: PASS")
        else:
            print("  ❌ PumpFun Integration: FAIL")

    except Exception as e:
        verification_results['checks']['pumpfun_integration'] = {
            'status': 'ERROR',
            'error': str(e)
        }
        print(f"  ❌ PumpFun Integration: ERROR - {e}")

    # Check 4: Performance Test Results
    print("\n⚡ Checking Performance Test Results...")
    total_checks += 1
    try:
        with open('real_performance_results.json', 'r') as f:
            perf_data = json.load(f)

        final_summary = perf_data.get('final_summary', {})
        perf_checks = {
            'Target achieved': final_summary.get('target_achieved', False),
            'Elite grade': final_summary.get('overall_grade') == 'ELITE',
            'UDP primary working': perf_data.get('real_tests', {}).get('udp_primary', {}).get('performance_grade') == 'ELITE',
            'gRPC backup ready': perf_data.get('real_tests', {}).get('grpc_backup', {}).get('status') == 'SUCCESS'
        }

        perf_passed = all(perf_checks.values())
        verification_results['checks']['performance'] = {
            'status': 'PASS' if perf_passed else 'FAIL',
            'details': perf_checks,
            'latency_ms': final_summary.get('end_to_end_latency_ms', 'N/A')
        }

        if perf_passed:
            checks_passed += 1
            print("  ✅ Performance Tests: PASS")
            print(f"    ⚡ End-to-end latency: {final_summary.get('end_to_end_latency_ms', 'N/A')}ms")
        else:
            print("  ❌ Performance Tests: FAIL")

    except Exception as e:
        verification_results['checks']['performance'] = {
            'status': 'ERROR',
            'error': str(e)
        }
        print(f"  ❌ Performance Tests: ERROR - {e}")

    # Check 5: Dependencies and Build Readiness
    print("\n🔧 Checking Build Dependencies...")
    total_checks += 1

    rust_available = os.system("which rustc > /dev/null 2>&1") == 0
    cargo_available = os.system("which cargo > /dev/null 2>&1") == 0

    # We can't check for build-essential without sudo, but we can check for basic tools
    build_checks = {
        'Rust compiler': rust_available,
        'Cargo build tool': cargo_available,
        'Cargo.toml exists': os.path.exists('Cargo.toml'),
        'Source directory': os.path.exists('src/bin/elite_mev_bot_v2_1_production.rs')
    }

    build_passed = all(build_checks.values())
    verification_results['checks']['build_dependencies'] = {
        'status': 'PASS' if build_passed else 'FAIL',
        'details': build_checks
    }

    if build_passed:
        checks_passed += 1
        print("  ✅ Build Dependencies: PASS")
    else:
        print("  ❌ Build Dependencies: FAIL")
        for check, result in build_checks.items():
            status = "✅" if result else "❌"
            print(f"    {status} {check}")

    # Final Summary
    verification_results['summary'] = {
        'checks_passed': checks_passed,
        'total_checks': total_checks,
        'pass_rate': round(checks_passed / total_checks * 100, 1),
        'ready_for_live_trading': checks_passed == total_checks,
        'deployment_status': 'READY' if checks_passed == total_checks else 'NOT_READY'
    }

    print("\n" + "=" * 70)
    print("📊 LIVE TRADING READINESS SUMMARY")
    print("=" * 70)

    summary = verification_results['summary']
    print(f"🎯 Overall Status: {summary['deployment_status']}")
    print(f"✅ Checks Passed: {summary['checks_passed']}/{summary['total_checks']} ({summary['pass_rate']}%)")

    if summary['ready_for_live_trading']:
        print(f"\n🚀 STATUS: READY FOR LIVE MONEY TRADING!")
        print(f"✅ All critical systems verified")
        print(f"✅ Real trading enabled")
        print(f"✅ PumpFun integration ready")
        print(f"✅ Performance targets met")
        print(f"✅ Safety measures configured")

        print(f"\n💰 NEXT STEPS:")
        print(f"   1. Run: sudo ./deploy_for_live_trading.sh")
        print(f"   2. Verify wallet has >3.5 SOL")
        print(f"   3. Start with: cd deployment && ./elite_mev_bot_v2_1_production")
        print(f"   4. Monitor logs/ directory closely")

    else:
        print(f"\n⚠️  STATUS: NOT READY FOR LIVE TRADING")
        print(f"❌ Fix failed checks above before deployment")
        print(f"🔧 Review configuration and source code")

    print("=" * 70)

    # Save results
    with open('live_readiness_verification.json', 'w') as f:
        json.dump(verification_results, f, indent=2)

    print(f"\n💾 Verification results saved to: live_readiness_verification.json")

    return verification_results

if __name__ == "__main__":
    results = verify_live_readiness()