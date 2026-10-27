import math
import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    for token in sys.stdin.read().split():
        print(math.factorial(int(token)))


main()
