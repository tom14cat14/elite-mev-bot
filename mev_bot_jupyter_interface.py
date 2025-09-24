#!/usr/bin/env python3
"""
Elite MEV Bot v2.1 Jupyter Interface
Provides Python interface to interact with Rust MEV bot from Jupyter notebooks
"""

import subprocess
import json
import os
import time
import threading
from pathlib import Path
from typing import Dict, List, Optional, Any

class MEVBotInterface:
    """Python interface for Elite MEV Bot v2.1 Production"""

    def __init__(self, bot_directory: str = "."):
        self.bot_directory = Path(bot_directory).resolve()
        self.bot_process = None
        self.is_running = False

    def get_bot_status(self) -> Dict[str, Any]:
        """Get current status of MEV bot files and executables"""
        status = {
            "directory": str(self.bot_directory),
            "executables": {},
            "rust_files": {},
            "config_files": {},
            "build_status": "unknown"
        }

        # Check for executables
        executables = [
            "elite_mev_standalone_test",
            "elite_mev_final_test"
        ]

        for exe in executables:
            exe_path = self.bot_directory / exe
            status["executables"][exe] = {
                "exists": exe_path.exists(),
                "executable": exe_path.is_file() and os.access(exe_path, os.X_OK),
                "size": exe_path.stat().st_size if exe_path.exists() else 0
            }

        # Check for Rust source files
        rust_files = list(self.bot_directory.glob("**/*.rs"))
        status["rust_files"] = {
            str(f.relative_to(self.bot_directory)): {
                "size": f.stat().st_size,
                "modified": f.stat().st_mtime
            } for f in rust_files
        }

        # Check for config files
        config_files = [".env", "Cargo.toml", "dashboard.html"]
        for config in config_files:
            config_path = self.bot_directory / config
            status["config_files"][config] = config_path.exists()

        return status

    def run_standalone_test(self) -> Dict[str, Any]:
        """Run the standalone MEV bot test"""
        exe_path = self.bot_directory / "elite_mev_standalone_test"

        if not exe_path.exists():
            return {
                "success": False,
                "error": f"Executable not found: {exe_path}",
                "output": "",
                "stderr": ""
            }

        try:
            print("🚀 Starting Elite MEV Bot v2.1 Standalone Test...")
            result = subprocess.run(
                [str(exe_path)],
                cwd=self.bot_directory,
                capture_output=True,
                text=True,
                timeout=30
            )

            return {
                "success": result.returncode == 0,
                "return_code": result.returncode,
                "output": result.stdout,
                "stderr": result.stderr,
                "execution_time": "< 30s"
            }

        except subprocess.TimeoutExpired:
            return {
                "success": False,
                "error": "Test timed out after 30 seconds",
                "output": "",
                "stderr": ""
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e),
                "output": "",
                "stderr": ""
            }

    def build_mev_bot(self, target: str = "elite_mev_bot_v2_1_production") -> Dict[str, Any]:
        """Build the specified MEV bot target"""
        try:
            print(f"🔨 Building {target}...")
            result = subprocess.run(
                ["cargo", "build", "--bin", target],
                cwd=self.bot_directory,
                capture_output=True,
                text=True,
                timeout=300  # 5 minutes timeout
            )

            return {
                "success": result.returncode == 0,
                "return_code": result.returncode,
                "output": result.stdout,
                "stderr": result.stderr,
                "target": target
            }

        except subprocess.TimeoutExpired:
            return {
                "success": False,
                "error": f"Build timed out after 5 minutes for target: {target}",
                "output": "",
                "stderr": ""
            }
        except Exception as e:
            return {
                "success": False,
                "error": str(e),
                "output": "",
                "stderr": ""
            }

    def check_dependencies(self) -> Dict[str, Any]:
        """Check if all required dependencies are available"""
        deps = {
            "cargo": {"cmd": ["cargo", "--version"], "required": True},
            "rustc": {"cmd": ["rustc", "--version"], "required": True},
            "git": {"cmd": ["git", "--version"], "required": False}
        }

        results = {}

        for dep, config in deps.items():
            try:
                result = subprocess.run(
                    config["cmd"],
                    capture_output=True,
                    text=True,
                    timeout=5
                )
                results[dep] = {
                    "available": result.returncode == 0,
                    "version": result.stdout.strip(),
                    "required": config["required"]
                }
            except Exception as e:
                results[dep] = {
                    "available": False,
                    "error": str(e),
                    "required": config["required"]
                }

        return results

    def get_build_targets(self) -> List[str]:
        """Get list of available build targets from Cargo.toml"""
        cargo_toml = self.bot_directory / "Cargo.toml"
        if not cargo_toml.exists():
            return []

        try:
            # Parse Cargo.toml to find binary targets
            with open(cargo_toml, 'r') as f:
                content = f.read()

            # Simple parsing - look for [[bin]] sections
            targets = []
            lines = content.split('\n')
            in_bin_section = False

            for line in lines:
                line = line.strip()
                if line == '[[bin]]':
                    in_bin_section = True
                elif line.startswith('[') and line != '[[bin]]':
                    in_bin_section = False
                elif in_bin_section and line.startswith('name'):
                    # Extract name = "target_name"
                    name = line.split('=')[1].strip().strip('"\'')
                    targets.append(name)

            return targets

        except Exception as e:
            print(f"Error parsing Cargo.toml: {e}")
            return []

    def monitor_performance(self, duration: int = 10) -> Dict[str, Any]:
        """Monitor system performance during bot operation"""
        import psutil

        metrics = {
            "cpu_percent": [],
            "memory_percent": [],
            "disk_io": [],
            "network_io": [],
            "duration": duration
        }

        print(f"📊 Monitoring system performance for {duration} seconds...")

        start_time = time.time()
        while time.time() - start_time < duration:
            metrics["cpu_percent"].append(psutil.cpu_percent())
            metrics["memory_percent"].append(psutil.virtual_memory().percent)

            # Sample every 0.5 seconds
            time.sleep(0.5)

        # Calculate averages
        metrics["avg_cpu"] = sum(metrics["cpu_percent"]) / len(metrics["cpu_percent"])
        metrics["avg_memory"] = sum(metrics["memory_percent"]) / len(metrics["memory_percent"])
        metrics["max_cpu"] = max(metrics["cpu_percent"])
        metrics["max_memory"] = max(metrics["memory_percent"])

        return metrics

def display_bot_status(status: Dict[str, Any]):
    """Display bot status in a readable format"""
    print("🤖 Elite MEV Bot v2.1 Status Report")
    print("=" * 50)

    print(f"📁 Directory: {status['directory']}")
    print()

    print("📋 Executables:")
    for exe, info in status["executables"].items():
        status_icon = "✅" if info["exists"] and info["executable"] else "❌"
        size_mb = info["size"] / (1024 * 1024) if info["size"] > 0 else 0
        print(f"  {status_icon} {exe}: {size_mb:.1f} MB")
    print()

    print("🦀 Rust Source Files:")
    for rust_file, info in sorted(status["rust_files"].items()):
        size_kb = info["size"] / 1024
        print(f"  📄 {rust_file}: {size_kb:.1f} KB")
    print()

    print("⚙️ Configuration Files:")
    for config, exists in status["config_files"].items():
        status_icon = "✅" if exists else "❌"
        print(f"  {status_icon} {config}")


# Convenience functions for Jupyter
def quick_test():
    """Quick test function for Jupyter notebooks"""
    bot = MEVBotInterface()
    print("🚀 Running Quick MEV Bot Test...")
    result = bot.run_standalone_test()

    if result["success"]:
        print("✅ Test completed successfully!")
        # Show last few lines of output
        lines = result["output"].split('\n')
        print("\n📊 Test Results Summary:")
        for line in lines[-10:]:
            if line.strip():
                print(f"  {line}")
    else:
        print("❌ Test failed!")
        print(f"Error: {result.get('error', 'Unknown error')}")
        if result.get("stderr"):
            print(f"STDERR: {result['stderr'][:500]}...")

def show_status():
    """Show current bot status"""
    bot = MEVBotInterface()
    status = bot.get_bot_status()
    display_bot_status(status)

def check_deps():
    """Check dependencies"""
    bot = MEVBotInterface()
    deps = bot.check_dependencies()

    print("🔍 Dependency Check")
    print("=" * 30)

    for dep, info in deps.items():
        status_icon = "✅" if info["available"] else "❌"
        required_text = "(Required)" if info["required"] else "(Optional)"

        if info["available"]:
            print(f"{status_icon} {dep}: {info.get('version', 'Available')} {required_text}")
        else:
            print(f"{status_icon} {dep}: Not available {required_text}")
            if "error" in info:
                print(f"    Error: {info['error']}")

if __name__ == "__main__":
    # Demo when run directly
    bot = MEVBotInterface()
    show_status()
    print()
    check_deps()
    print()
    quick_test()