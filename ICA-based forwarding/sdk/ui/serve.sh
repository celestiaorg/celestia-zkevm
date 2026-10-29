#!/bin/bash
# Simple HTTP server to serve the UI
# Usage: ./serve.sh [port]

PORT=${1:-3000}
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🚀 Starting ICA Forwarding SDK Tester UI..."
echo "   Open http://localhost:$PORT in your browser"
echo ""
echo "   Press Ctrl+C to stop"
echo ""

cd "$SCRIPT_DIR"

# Try python3 first, then python
if command -v python3 &> /dev/null; then
    python3 -m http.server $PORT
elif command -v python &> /dev/null; then
    python -m http.server $PORT
else
    echo "Error: Python is required to run the server"
    echo "Install Python or use any other static file server"
    exit 1
fi

