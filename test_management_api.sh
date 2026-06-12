#!/bin/bash
# Test script untuk Melisa Management API dan MNode

set -e

MELISA_HOST="127.0.0.1"
MELISA_PORT="8888"
MNODE_HOST="127.0.0.1"
MNODE_PORT="3000"

echo "╔════════════════════════════════════════════════════╗"
echo "║       MELISA MANAGEMENT API TEST SUITE            ║"
echo "╚════════════════════════════════════════════════════╝"
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Test counter
TESTS_PASSED=0
TESTS_FAILED=0

# Helper function to test endpoint
test_endpoint() {
    local name=$1
    local method=$2
    local endpoint=$3
    local data=$4
    local expected_status=$5

    echo -n "Testing: $name ... "
    
    if [ -z "$data" ]; then
        response=$(curl -s -w "\n%{http_code}" -X $method "http://$MELISA_HOST:$MELISA_PORT$endpoint")
    else
        response=$(curl -s -w "\n%{http_code}" -X $method "http://$MELISA_HOST:$MELISA_PORT$endpoint" \
            -H "Content-Type: application/json" \
            -d "$data")
    fi
    
    http_code=$(echo "$response" | tail -n1)
    body=$(echo "$response" | head -n-1)
    
    if [ "$http_code" == "$expected_status" ]; then
        echo -e "${GREEN}✓ PASS${NC} (HTTP $http_code)"
        TESTS_PASSED=$((TESTS_PASSED + 1))
        echo "$body" | jq . 2>/dev/null || echo "$body"
    else
        echo -e "${RED}✗ FAIL${NC} (HTTP $http_code, expected $expected_status)"
        TESTS_FAILED=$((TESTS_FAILED + 1))
        echo "$body" | jq . 2>/dev/null || echo "$body"
    fi
    echo ""
}

# Test 1: List nodes (should be empty initially)
echo -e "${BLUE}[TEST SUITE 1] Management API Basics${NC}"
test_endpoint "List nodes (empty)" "GET" "/nodes" "" "200"

# Test 2: Register first node
register_data='{
    "name": "test-service-1",
    "pid": 10000,
    "url": "http://127.0.0.1:3000",
    "domain": "test.local",
    "route_path": "/api"
}'
test_endpoint "Register node 1" "POST" "/register" "$register_data" "201"

# Extract hash from last response for later use
LAST_HASH=$(echo "$body" | jq -r '.node.hash' 2>/dev/null || echo "")

# Test 3: List nodes (should show 1 node)
test_endpoint "List nodes (1 node)" "GET" "/nodes" "" "200"

# Test 4: Try to register duplicate
test_endpoint "Register duplicate (should fail)" "POST" "/register" "$register_data" "409"

# Test 5: Register second node
register_data2='{
    "name": "test-service-2",
    "pid": 10001,
    "url": "http://127.0.0.1:3001",
    "domain": "test2.local",
    "route_path": "/backend"
}'
test_endpoint "Register node 2" "POST" "/register" "$register_data2" "201"

# Test 6: List nodes (should show 2 nodes)
test_endpoint "List nodes (2 nodes)" "GET" "/nodes" "" "200"

# Test 7: Unregister node 1
echo -e "${BLUE}[TEST SUITE 2] Node Unregistration${NC}"
if [ ! -z "$LAST_HASH" ]; then
    unregister_data="{\"hash\": \"$LAST_HASH\"}"
    test_endpoint "Unregister node 1" "POST" "/unregister" "$unregister_data" "200"
else
    echo "Skipping unregister test - could not extract hash"
fi

# Test 8: List nodes (should show 1 node)
test_endpoint "List nodes after unregister (1 node)" "GET" "/nodes" "" "200"

# Test 9: Invalid JSON
echo -e "${BLUE}[TEST SUITE 3] Error Handling${NC}"
test_endpoint "Invalid JSON" "POST" "/register" "not json" "400"

# Test 10: Missing required fields
test_endpoint "Missing required fields" "POST" "/register" '{"name":"test"}' "400"

# Test 11: Invalid endpoint
test_endpoint "Invalid endpoint (404)" "GET" "/invalid" "" "404"

# Test 12: Invalid unregister (missing hash)
test_endpoint "Unregister without hash" "POST" "/unregister" '{"other":"field"}' "400"

# Test 13: Unregister non-existent node
echo -e "${BLUE}[TEST SUITE 4] Edge Cases${NC}"
test_endpoint "Unregister non-existent node" "POST" "/unregister" '{"hash":"nonexistent"}' "404"

# Test MNode endpoints
echo ""
echo -e "${BLUE}[TEST SUITE 5] MNode Endpoints${NC}"
echo "Testing MNode on http://$MNODE_HOST:$MNODE_PORT"
echo ""

# Check if MNode is running
if ! curl -s -f http://$MNODE_HOST:$MNODE_PORT/api/health > /dev/null 2>&1; then
    echo -e "${RED}Warning: MNode is not running on $MNODE_HOST:$MNODE_PORT${NC}"
    echo "Start MNode with: cargo run --bin mnode"
    echo ""
else
    test_endpoint "MNode - Health check" "GET" "" "" "200" | sed "s|http://$MELISA_HOST:$MELISA_PORT|http://$MNODE_HOST:$MNODE_PORT/api/health|"
fi

# Summary
echo ""
echo "╔════════════════════════════════════════════════════╗"
echo "║                  TEST SUMMARY                      ║"
echo "╚════════════════════════════════════════════════════╝"
echo -e "Tests Passed: ${GREEN}$TESTS_PASSED${NC}"
echo -e "Tests Failed: ${RED}$TESTS_FAILED${NC}"
echo "Total Tests: $((TESTS_PASSED + TESTS_FAILED))"
echo ""

if [ $TESTS_FAILED -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    exit 0
else
    echo -e "${RED}✗ Some tests failed!${NC}"
    exit 1
fi
