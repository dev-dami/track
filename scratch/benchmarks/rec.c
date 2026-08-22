#include <stdio.h>
long long fib(long long n){ if(n<=1) return n; return fib(n-1)+fib(n-2); }
int main(){ long long r=fib(38); printf("%lld\n", r); return 0; }
