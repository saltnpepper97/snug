#!/usr/bin/env bash

# Kill any previous instances
pkill -x snug 2>/dev/null
pkill -x waybar 2>/dev/null

# Start new instances detached
snug >/dev/null 2>&1 &
waybar >/dev/null 2>&1 &

# Exit script
exit 0
