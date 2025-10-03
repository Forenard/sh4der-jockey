#!/usr/bin/env python3
"""
Simple OSC test script for Sh4der Jockey

Requirements:
    pip install python-osc

Usage:
    python test_osc.py

This script sends OSC messages to test the brightness uniform in the shader.
"""

import time
import math
from pythonosc import udp_client

def main():
    # Connect to OSC receiver (should match the port in pipeline.yaml)
    client = udp_client.SimpleUDPClient("127.0.0.1", 9000)

    print("Starting OSC test for Sh4der Jockey...")
    print("Sending brightness values to /volume")
    print("Press Ctrl+C to stop")

    try:
        t = 0
        while True:
            # Generate a oscillating brightness value between 0 and 1
            brightness = 0.5 + 0.5 * math.sin(t)

            # Send OSC message
            client.send_message("/volume", brightness)
            print(f"Sent: /volume {brightness:.3f}")

            # Wait and increment time
            time.sleep(0.1)
            t += 0.1

    except KeyboardInterrupt:
        print("\nOSC test stopped.")

if __name__ == "__main__":
    main()