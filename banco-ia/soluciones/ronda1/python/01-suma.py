import sys

total = 0
for linea in sys.stdin.read().split():
    total += int(linea)
print(total)
