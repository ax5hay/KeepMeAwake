"""Keep the system active by simulating an Alt key press every 15 minutes.

The original, kept as a souvenir. Everything in this repository grew out of
these twenty lines. It still works; it is just no longer the way.

    Akshay Bajpai, akshaybajpai.com
"""

import ctypes
import time

VK_MENU = 0x12  # Alt key
KEYEVENTF_KEYUP = 0x0002
INTERVAL_SECONDS = 15 * 60

user32 = ctypes.windll.user32


def press_alt() -> None:
    user32.keybd_event(VK_MENU, 0, 0, 0)
    user32.keybd_event(VK_MENU, 0, KEYEVENTF_KEYUP, 0)


def main() -> None:
    print("KeepMeAwake running. Pressing Alt every 15 minutes. Ctrl+C to stop.")
    try:
        while True:
            press_alt()
            print(f"Alt key pressed at {time.strftime('%H:%M:%S')}")
            time.sleep(INTERVAL_SECONDS)
    except KeyboardInterrupt:
        print("\nStopped.")


if __name__ == "__main__":
    main()
