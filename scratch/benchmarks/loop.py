sum_val = 0
for i in range(100000000):
    sum_val += i & 7
print(sum_val)
