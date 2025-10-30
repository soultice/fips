#!/bin/bash

# System Callout Plugin - Demo Script
# This script builds and tests the system callout plugin to demonstrate
# the power and security implications of FIPS plugins.

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${RED}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${RED}║  ⚠️  SECURITY WARNING - SYSTEM CALLOUT PLUGIN DEMONSTRATION  ⚠️   ║${NC}"
echo -e "${RED}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}This plugin demonstrates that FIPS plugins can:${NC}"
echo -e "  • Execute arbitrary system commands"
echo -e "  • Read any file on the filesystem"
echo -e "  • Access environment variables (including secrets)"
echo -e "  • Make HTTP requests (SSRF potential)"
echo ""
echo -e "${RED}DO NOT use this plugin in production!${NC}"
echo -e "${RED}DO NOT expose to untrusted networks!${NC}"
echo ""
read -p "Press Enter to continue with the demo..."

echo ""
echo -e "${BLUE}Step 1: Building the plugin...${NC}"
cd "$PROJECT_ROOT/plugins/system_callout"
cargo build --release

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓ Plugin built successfully${NC}"
else
    echo -e "${RED}✗ Plugin build failed${NC}"
    exit 1
fi

# Determine library extension based on OS
if [[ "$OSTYPE" == "darwin"* ]]; then
    LIB_EXT="dylib"
else
    LIB_EXT="so"
fi

PLUGIN_PATH="$PROJECT_ROOT/plugins/system_callout/target/release/libsystem_callout.$LIB_EXT"

if [ -f "$PLUGIN_PATH" ]; then
    echo -e "${GREEN}✓ Plugin file exists: $PLUGIN_PATH${NC}"
else
    echo -e "${RED}✗ Plugin file not found: $PLUGIN_PATH${NC}"
    exit 1
fi

# Copy plugin to _plugins directory
mkdir -p "$PROJECT_ROOT/_plugins"
cp "$PLUGIN_PATH" "$PROJECT_ROOT/_plugins/"
echo -e "${GREEN}✓ Plugin copied to _plugins directory${NC}"

echo ""
echo -e "${BLUE}Step 2: Starting FIPS server (in background)...${NC}"
cd "$PROJECT_ROOT"

# Kill any existing FIPS instances
pkill -f "target/release/fips" 2>/dev/null || true
pkill -f "target/debug/fips" 2>/dev/null || true
sleep 1

# Start FIPS in release mode
cargo run --release -- -c ./nconfig-test/ > /tmp/fips-demo.log 2>&1 &
FIPS_PID=$!

echo -e "${GREEN}✓ FIPS started (PID: $FIPS_PID)${NC}"
echo "  Log file: /tmp/fips-demo.log"

# Wait for server to start
echo -n "  Waiting for server to be ready"
for i in {1..30}; do
    if curl -s http://localhost:8888/ > /dev/null 2>&1; then
        echo -e " ${GREEN}✓${NC}"
        break
    fi
    echo -n "."
    sleep 0.5
done

if ! curl -s http://localhost:8888/ > /dev/null 2>&1; then
    echo -e " ${RED}✗${NC}"
    echo -e "${RED}Server failed to start. Check /tmp/fips-demo.log${NC}"
    kill $FIPS_PID 2>/dev/null || true
    exit 1
fi

echo ""
echo -e "${BLUE}Step 3: Testing SAFE operations...${NC}"
echo ""
echo -e "${YELLOW}=== Test 1: System Information (date, whoami, uptime) ===${NC}"
curl -s http://localhost:8888/demo/safe | jq '.' 2>/dev/null || curl -s http://localhost:8888/demo/safe
echo ""

sleep 1

echo ""
echo -e "${RED}=== Test 2: DANGEROUS Operations ===${NC}"
echo -e "${YELLOW}This demonstrates reading files, env vars, and making HTTP requests${NC}"
curl -s http://localhost:8888/demo/dangerous | jq '.' 2>/dev/null || curl -s http://localhost:8888/demo/dangerous
echo ""

sleep 1

echo ""
echo -e "${RED}=== Test 3: Command Injection Risk ===${NC}"
echo -e "${YELLOW}This shows how an attacker could list sensitive directories${NC}"
curl -s http://localhost:8888/demo/injection | jq '.' 2>/dev/null || curl -s http://localhost:8888/demo/injection
echo ""

echo ""
echo -e "${BLUE}Step 4: Cleanup${NC}"
kill $FIPS_PID 2>/dev/null || true
echo -e "${GREEN}✓ FIPS server stopped${NC}"

echo ""
echo -e "${RED}╔════════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${RED}║                      SECURITY IMPLICATIONS                         ║${NC}"
echo -e "${RED}╚════════════════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "${YELLOW}What you just saw:${NC}"
echo "  ✓ Plugins can execute ANY system command"
echo "  ✓ Plugins can read ANY file (including secrets)"
echo "  ✓ Plugins can access environment variables"
echo "  ✓ Plugins can make HTTP requests (SSRF risk)"
echo ""
echo -e "${RED}Attack scenarios:${NC}"
echo "  • Command injection: rm -rf / or exfiltration commands"
echo "  • Data theft: Reading /etc/passwd, SSH keys, credentials"
echo "  • SSRF: Accessing internal APIs, cloud metadata endpoints"
echo "  • Lateral movement: Using compromised server to attack others"
echo ""
echo -e "${GREEN}Mitigation:${NC}"
echo "  1. Only use trusted plugins"
echo "  2. Never allow user input in plugin args"
echo "  3. Run FIPS with minimal privileges"
echo "  4. Use containers/sandboxing"
echo "  5. Audit all plugin configurations"
echo "  6. Monitor and log plugin executions"
echo ""
echo -e "${BLUE}Full security documentation:${NC}"
echo "  • plugins/system_callout/SECURITY.md"
echo "  • README.md (Security Warning section)"
echo ""
echo -e "${YELLOW}Remember: Plugin configuration files are as dangerous as executable code!${NC}"
echo ""
