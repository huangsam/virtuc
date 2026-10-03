#include <stdio.h>

float add(float a, float b) {
    return a + b;
}

float sub(float a, float b) {
    return a - b;
}

float mul(float a, float b) {
    return a * b;
}

float div(float a, float b) {
    return a / b;
}

int main() {
    float x = 10.5;
    float y = 2.5;

    float a = add(x, y); // 13.0
    float s = sub(x, y); // 8.0
    float m = mul(x, y); // 26.25
    float d = div(x, y); // 4.2
    float neg = -x;      // -10.5

    int cmp_lt = (x < y);    // 0
    int cmp_le = (x <= 10.5); // 1
    int cmp_gt = (x > y);    // 1
    int cmp_ge = (y >= 2.5);  // 1
    int cmp_eq = (x == 10.5); // 1
    int cmp_ne = (x != y);   // 1

    // Test float in if-condition
    int cond_result = 0;
    if (x > y) {
        cond_result = 42;
    }

    printf("a=%.1f s=%.1f m=%.2f d=%.1f neg=%.1f\n", a, s, m, d, neg);
    printf("cmp=%d%d%d%d%d%d cond=%d\n", cmp_lt, cmp_le, cmp_gt, cmp_ge, cmp_eq, cmp_ne, cond_result);
    return cond_result;
}
