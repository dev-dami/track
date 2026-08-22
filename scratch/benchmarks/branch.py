sum_val=0
for i in range(50000000):
    if (i & 1)==0: sum_val+=i
    else: sum_val-=i
print(sum_val)
