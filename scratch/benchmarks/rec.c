#include <stdio.h>
long long fib(long long n){ volatile long long v=n; if(v<=1) return v; return fib(v-1)+fib(v-2); }
int main(){ volatile long long r=fib(38); printf("%lld\n", r); return 0; }
