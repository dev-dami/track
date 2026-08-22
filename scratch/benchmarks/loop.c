#include <stdio.h>
int main() {
    volatile long long sum = 0;
    for (long long i = 0; i < 100000000; i++) sum += (i & 7);
    printf("%lld\n", sum);
    return 0;
}
