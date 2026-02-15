#!/usr/bin/env bash
# End-to-end WAF tests (run against a live docker-compose stack)
set -euo pipefail

WAF_URL="${WAF_URL:-http://localhost:8080}"
ADMIN_URL="${ADMIN_URL:-http://localhost:9090}"

pass=0
fail=0

check() {
    local desc="$1"
    local expected_status="$2"
    shift 2
    local actual_status
    actual_status=$(curl -s -o /dev/null -w "%{http_code}" "$@")
    if [ "$actual_status" = "$expected_status" ]; then
        echo "  PASS: $desc (got $actual_status)"
        pass=$((pass + 1))
    else
        echo "  FAIL: $desc (expected $expected_status, got $actual_status)"
        fail=$((fail + 1))
    fi
}

echo "=== Layer 7 WAF E2E Tests ==="
echo

echo "--- Health & Admin ---"
check "Admin health endpoint" "200" "$ADMIN_URL/api/health"
check "Admin metrics endpoint" "200" "$ADMIN_URL/api/metrics"
check "Admin stats endpoint" "200" "$ADMIN_URL/api/stats"
check "Admin config endpoint" "200" "$ADMIN_URL/api/config"
check "Admin rules endpoint" "200" "$ADMIN_URL/api/rules"
check "Admin logs endpoint" "200" "$ADMIN_URL/api/logs"

echo
echo "--- Clean Requests (should pass) ---"
check "Normal GET request" "200" "$WAF_URL/"
check "Normal GET with path" "200" "$WAF_URL/api/test"
check "GET with query params" "200" "$WAF_URL/?page=1&size=10"

echo
echo "--- WAF Blocked Requests ---"
check "SQL Injection in query" "403" "$WAF_URL/?id=1%20OR%201=1"
check "XSS in path" "403" "$WAF_URL/%3Cscript%3Ealert(1)%3C/script%3E"
check "SQL Injection UNION" "403" "$WAF_URL/?q=1%20UNION%20SELECT%20*%20FROM%20users"
check "Path traversal" "403" "$WAF_URL/../../etc/passwd"
check "XSS in header" "403" -H "X-Custom: <script>alert(1)</script>" "$WAF_URL/"

echo
echo "=== Results: $pass passed, $fail failed ==="
[ "$fail" -eq 0 ]
