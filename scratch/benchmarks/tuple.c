#include <stdio.h>
typedef struct{long long a,b;} Pair;
int main(){
    volatile long long sum=0;
    for(long long i=0;i<10000000;i++){ volatile Pair t={i,i+1}; sum+=t.a+t.b; }
    printf("%lld\n", sum); return 0;
}
