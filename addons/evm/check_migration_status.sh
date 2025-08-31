#!/bin/bash
# Script to check test migration status for EVM addon

echo "EVM Test Migration Status Report"
echo "================================"
echo ""

# Count total test functions
TOTAL_TESTS=$(grep -r "^\s*#\[test\]" src/tests --include="*.rs" | wc -l)
echo "Total test functions: $TOTAL_TESTS"

# Count tests using ProjectTestHarness (migrated to txtx)
MIGRATED_TESTS=$(grep -r "ProjectTestHarness" src/tests --include="*.rs" -A5 | grep -c "#\[test\]")
echo "Tests using ProjectTestHarness: $MIGRATED_TESTS"

# Count tests still using direct Alloy/other approaches
LEGACY_TESTS=$((TOTAL_TESTS - MIGRATED_TESTS))
echo "Tests not yet migrated: $LEGACY_TESTS"

# Calculate percentage
if [ $TOTAL_TESTS -gt 0 ]; then
    PERCENTAGE=$((MIGRATED_TESTS * 100 / TOTAL_TESTS))
    echo "Migration progress: ${PERCENTAGE}%"
fi

echo ""
echo "Fixture Statistics:"
echo "-------------------"

# Count fixtures
INTEGRATION_FIXTURES=$(find fixtures/integration -name "*.tx" 2>/dev/null | wc -l)
PARSING_FIXTURES=$(find fixtures/parsing -name "*.tx" 2>/dev/null | wc -l)
TOTAL_FIXTURES=$((INTEGRATION_FIXTURES + PARSING_FIXTURES))

echo "Integration fixtures: $INTEGRATION_FIXTURES"
echo "Parsing fixtures: $PARSING_FIXTURES"
echo "Total fixtures: $TOTAL_FIXTURES"

echo ""
echo "Test Files Overview:"
echo "--------------------"

# List test files with their test counts
for file in src/tests/*.rs src/tests/integration/*.rs; do
    if [ -f "$file" ]; then
        TEST_COUNT=$(grep -c "^\s*#\[test\]" "$file")
        MIGRATED=$(grep -c "ProjectTestHarness" "$file")
        FILENAME=$(basename "$file")
        printf "%-40s: %3d tests (%d migrated)\n" "$FILENAME" "$TEST_COUNT" "$MIGRATED"
    fi
done

echo ""
echo "Files with Inline Runbooks (Anti-pattern):"
echo "-------------------------------------------"

# Check for inline runbooks (r#" pattern in tests)
INLINE_COUNT=$(grep -r 'r#"' src/tests --include="*.rs" | grep -v "// Skip" | grep -v "fixture" | wc -l)
if [ $INLINE_COUNT -gt 0 ]; then
    echo "⚠️  Found $INLINE_COUNT potential inline runbooks:"
    grep -r 'r#"' src/tests --include="*.rs" -l | grep -v "fixture" | while read file; do
        echo "  - $file"
    done
else
    echo "✅ No inline runbooks found!"
fi

echo ""
echo "Next Steps:"
echo "-----------"
echo "1. Focus on high-priority user-facing actions (send_eth, deploy_contract)"
echo "2. Migrate error handling tests for better user experience"
echo "3. Update codec tests to use fixtures"
echo "4. Complete transaction and deployment test migrations"
echo ""
echo "Run this script periodically to track progress!"