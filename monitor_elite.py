#!/usr/bin/env python3
"""
ELITE MEV Bot Real-time Performance Monitor
Provides advanced analytics and optimization suggestions
"""

import time
import json
import re
import sys
import os
from datetime import datetime, timedelta
from collections import defaultdict, deque
import subprocess

class MEVPerformanceMonitor:
    def __init__(self):
        self.start_time = datetime.now()
        self.metrics = {
            'total_profit': 0.0,
            'total_trades': 0,
            'successful_trades': 0,
            'failed_trades': 0,
            'avg_profit_per_trade': 0.0,
            'profit_rate_per_hour': 0.0,
            'success_rate': 0.0,
            'best_trade': 0.0,
            'worst_trade': 0.0,
            'current_streak': 0,
            'best_streak': 0,
            'recent_profits': deque(maxlen=50),
            'hourly_profits': defaultdict(float),
        }
        self.risk_recommendations = []

    def parse_log_line(self, line):
        """Parse MEV bot log lines for performance data"""
        # Look for profit patterns
        profit_pattern = r'Profit: ([\d.]+) SOL'
        trade_pattern = r'Trade executed.*profit: ([\d.]+) SOL'
        opportunity_pattern = r'Opportunity.*executed.*profit: ([\d.]+) SOL'

        profit_match = re.search(profit_pattern, line)
        if profit_match:
            profit = float(profit_match.group(1))
            self.update_profit_metrics(profit)

        # Look for trade execution
        if 'executed successfully' in line and 'profit' in line:
            self.metrics['successful_trades'] += 1
            self.metrics['current_streak'] += 1
            self.metrics['best_streak'] = max(self.metrics['best_streak'], self.metrics['current_streak'])

        elif 'failed' in line and 'trade' in line:
            self.metrics['failed_trades'] += 1
            self.metrics['current_streak'] = 0

    def update_profit_metrics(self, profit):
        """Update profit-related metrics"""
        self.metrics['total_profit'] += profit
        self.metrics['total_trades'] += 1
        self.metrics['recent_profits'].append(profit)

        # Update best/worst trades
        self.metrics['best_trade'] = max(self.metrics['best_trade'], profit)
        if self.metrics['worst_trade'] == 0.0:
            self.metrics['worst_trade'] = profit
        else:
            self.metrics['worst_trade'] = min(self.metrics['worst_trade'], profit)

        # Calculate averages
        if self.metrics['total_trades'] > 0:
            self.metrics['avg_profit_per_trade'] = self.metrics['total_profit'] / self.metrics['total_trades']

        # Calculate hourly rate
        runtime_hours = (datetime.now() - self.start_time).total_seconds() / 3600
        if runtime_hours > 0:
            self.metrics['profit_rate_per_hour'] = self.metrics['total_profit'] / runtime_hours

        # Calculate success rate
        total_attempts = self.metrics['successful_trades'] + self.metrics['failed_trades']
        if total_attempts > 0:
            self.metrics['success_rate'] = (self.metrics['successful_trades'] / total_attempts) * 100

        # Store hourly profits
        current_hour = datetime.now().hour
        self.metrics['hourly_profits'][current_hour] += profit

    def generate_performance_report(self):
        """Generate comprehensive performance analysis"""
        runtime = datetime.now() - self.start_time
        runtime_str = str(runtime).split('.')[0]  # Remove microseconds

        print("\n" + "="*80)
        print("🚀 ELITE MEV BOT PERFORMANCE DASHBOARD")
        print("="*80)
        print(f"⏱️  Runtime: {runtime_str}")
        print(f"📊 Total Profit: {self.metrics['total_profit']:.4f} SOL")
        print(f"💰 Profit Rate: {self.metrics['profit_rate_per_hour']:.3f} SOL/hour")
        print(f"🎯 Success Rate: {self.metrics['success_rate']:.1f}%")
        print(f"📈 Trades: {self.metrics['successful_trades']} successful, {self.metrics['failed_trades']} failed")
        print(f"💎 Average/Trade: {self.metrics['avg_profit_per_trade']:.4f} SOL")
        print(f"🔥 Best Trade: {self.metrics['best_trade']:.4f} SOL")
        print(f"⚡ Current Streak: {self.metrics['current_streak']} (Best: {self.metrics['best_streak']})")

        # Performance analysis
        print("\n📊 PERFORMANCE ANALYSIS:")
        self.analyze_performance()

        # Optimization recommendations
        print("\n💡 OPTIMIZATION RECOMMENDATIONS:")
        self.generate_recommendations()

        # Recent performance trend
        if len(self.metrics['recent_profits']) >= 10:
            recent_avg = sum(list(self.metrics['recent_profits'])[-10:]) / 10
            print(f"\n📈 Recent Trend (last 10 trades): {recent_avg:.4f} SOL avg")

        print("="*80)

    def analyze_performance(self):
        """Analyze current performance levels"""
        profit_rate = self.metrics['profit_rate_per_hour']
        success_rate = self.metrics['success_rate']

        # Performance rating
        if profit_rate > 3.0 and success_rate > 80:
            print("🔥 EXCEPTIONAL: Elite performance - maintain current settings")
        elif profit_rate > 2.0 and success_rate > 70:
            print("✅ EXCELLENT: Strong performance - minor optimizations possible")
        elif profit_rate > 1.0 and success_rate > 60:
            print("📈 GOOD: Solid performance - consider parameter tuning")
        elif profit_rate > 0.5 and success_rate > 40:
            print("⚠️  MODERATE: Room for improvement - review strategy")
        else:
            print("🔧 NEEDS OPTIMIZATION: Consider adjusting risk parameters")

    def generate_recommendations(self):
        """Generate optimization recommendations based on performance"""
        recommendations = []

        profit_rate = self.metrics['profit_rate_per_hour']
        success_rate = self.metrics['success_rate']
        avg_trade = self.metrics['avg_profit_per_trade']

        # Success rate based recommendations
        if success_rate < 40:
            recommendations.append("🎯 Increase MIN_PROFIT_SOL threshold (current too aggressive)")
            recommendations.append("⚡ Increase timeout_ms for better execution (reduce speed)")
            recommendations.append("🔄 Consider lowering RISK_LEVEL")

        elif success_rate > 85:
            recommendations.append("🚀 Decrease MIN_PROFIT_SOL threshold (capture more opportunities)")
            recommendations.append("⚡ Decrease timeout_ms for faster execution")
            recommendations.append("📈 Consider increasing RISK_LEVEL")

        # Profit rate based recommendations
        if profit_rate < 0.5:
            recommendations.append("💰 Increase CAPITAL_SOL for larger position sizes")
            recommendations.append("🎯 Review market timing - consider different trading hours")

        elif profit_rate > 3.0:
            recommendations.append("🔥 Excellent rate! Consider increasing position size")
            recommendations.append("📊 Monitor for market condition changes")

        # Trade size recommendations
        if avg_trade < 0.1:
            recommendations.append("💎 Consider increasing minimum profit threshold")
            recommendations.append("🔍 Focus on higher volume opportunities")

        # Display recommendations
        if recommendations:
            for rec in recommendations:
                print(f"  {rec}")
        else:
            print("  🎯 Performance is well-balanced - continue monitoring")

    def monitor_real_time(self, log_file=None):
        """Monitor MEV bot performance in real-time"""
        print("🔍 Starting real-time MEV performance monitoring...")
        print("📊 Updates every 30 seconds | Press Ctrl+C to stop")

        if log_file and os.path.exists(log_file):
            # Monitor specific log file
            with open(log_file, 'r') as f:
                # Read existing content
                for line in f:
                    self.parse_log_line(line.strip())

                # Monitor new content
                while True:
                    line = f.readline()
                    if line:
                        self.parse_log_line(line.strip())
                    else:
                        time.sleep(1)
        else:
            # Monitor live process (if running)
            print("⚠️  Live monitoring mode - watching for MEV bot process")
            update_counter = 0
            while True:
                time.sleep(1)
                update_counter += 1

                # Show dashboard every 30 seconds
                if update_counter % 30 == 0:
                    os.system('clear')  # Clear screen
                    self.generate_performance_report()
                    print("\n🔄 Monitoring continues... (Ctrl+C to stop)")

def main():
    monitor = MEVPerformanceMonitor()

    # Check for log file argument
    log_file = None
    if len(sys.argv) > 1:
        log_file = sys.argv[1]
        if not os.path.exists(log_file):
            print(f"❌ Log file not found: {log_file}")
            print("💡 Usage: python3 monitor_elite.py [log_file_path]")
            sys.exit(1)

    try:
        monitor.monitor_real_time(log_file)
    except KeyboardInterrupt:
        print("\n\n🛑 Monitoring stopped")
        monitor.generate_performance_report()
        print("\n👋 Monitor session ended")

if __name__ == "__main__":
    main()