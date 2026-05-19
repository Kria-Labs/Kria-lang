#!/bin/bash

# Kria Language Benchmark Suite
# Measures execution time of various benchmark tests

KRIA_BINARY="./target/release/kria"
BENCH_DIR="./benchmarks"
RESULTS_FILE="benchmark_results.txt"

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check if binary exists
if [ ! -f "$KRIA_BINARY" ]; then
    echo -e "${RED}Error: $KRIA_BINARY not found${NC}"
    echo "Building release binary..."
    cargo build --release
    if [ ! -f "$KRIA_BINARY" ]; then
        echo -e "${RED}Failed to build Kria. Exiting.${NC}"
        exit 1
    fi
fi

# Check if benchmarks directory exists
if [ ! -d "$BENCH_DIR" ]; then
    echo -e "${RED}Error: $BENCH_DIR directory not found${NC}"
    exit 1
fi

echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}    Kria Language Benchmark Suite${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Initialize results
> "$RESULTS_FILE"
declare -A times
total_time=0

# Run benchmarks
bench_count=0
for bench_file in "$BENCH_DIR"/*.krx; do
    if [ -f "$bench_file" ]; then
        bench_count=$((bench_count + 1))
        bench_name=$(basename "$bench_file" .krx)
        
        echo -n "Running ${bench_name}... "
        
        # Measure execution time using /usr/bin/time
        start_time=$(date +%s.%N)
        output=$("$KRIA_BINARY" "$bench_file" 2>&1)
        exit_code=$?
        end_time=$(date +%s.%N)
        
        elapsed=$(echo "$end_time - $start_time" | bc)
        times[$bench_name]=$elapsed
        total_time=$(echo "$total_time + $elapsed" | bc)
        
        if [ $exit_code -eq 0 ]; then
            echo -e "${GREEN}✓${NC} ${elapsed}s (output: $output)"
            echo "$bench_name: ${elapsed}s - Output: $output" >> "$RESULTS_FILE"
        else
            echo -e "${RED}✗${NC} ${elapsed}s (error)"
            echo "$bench_name: ${elapsed}s - ERROR" >> "$RESULTS_FILE"
        fi
    fi
done

echo ""
echo -e "${BLUE}========================================${NC}"
echo -e "${BLUE}           Benchmark Results${NC}"
echo -e "${BLUE}========================================${NC}"
echo ""

# Print results table
printf "%-30s %15s\n" "Test Name" "Time (seconds)"
printf "%-30s %15s\n" "---" "---"

for bench_name in "${!times[@]}"; do
    printf "%-30s %15s\n" "$bench_name" "${times[$bench_name]}"
done | sort

echo ""
printf "%-30s %15s\n" "TOTAL" "${total_time}s"
printf "%-30s %15s\n" "Tests Run" "$bench_count"

if [ $bench_count -gt 0 ]; then
    average=$(echo "scale=6; $total_time / $bench_count" | bc)
    printf "%-30s %15s\n" "Average" "${average}s"
fi

echo ""
echo -e "${BLUE}========================================${NC}"
echo ""
echo -e "${GREEN}Results saved to: $RESULTS_FILE${NC}"
