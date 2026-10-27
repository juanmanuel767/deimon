import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    tokens = sys.stdin.read().split()
    for k in range(0, len(tokens) - 1, 2):
        print(int(tokens[k]) * int(tokens[k + 1]))


main()
