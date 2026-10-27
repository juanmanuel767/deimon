import itertools
import sys


def main():
    sys.stdin.reconfigure(encoding="utf-8")
    sys.stdout.reconfigure(encoding="utf-8")
    palabra = sys.stdin.read().strip()
    if not palabra:
        return
    for permutacion in itertools.permutations(sorted(palabra)):
        print("".join(permutacion))


main()
