#include <stdio.h>
int main(){
    long long sum=0;
    for(long long i=0;i<50000000;i++){ if((i&1)==0) sum+=i; else sum-=i; }
    printf("%lld\n", sum); return 0;
}
