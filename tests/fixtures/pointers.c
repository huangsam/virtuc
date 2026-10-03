#include <stdio.h>

void swap(int* a, int* b) {
    int tmp = *a;
    *a = *b;
    *b = tmp;
}

int main() {
    // 1. Basic pointer and dereference assignment
    int val = 10;
    int* p = &val;
    *p = 25;

    // 2. Pass-by-pointer mutation
    int a = 1;
    int b = 2;
    swap(&a, &b);
    // now a == 2, b == 1

    // 3. Pointer to array decay, indexing, and pointer arithmetic
    int arr[3];
    arr[0] = 10;
    arr[1] = 20;
    arr[2] = 30;

    int* arr_ptr = arr;
    arr_ptr[1] = 5; // arr[1] becomes 5
    int elem2 = *(arr_ptr + 2); // 30

    // Pointer comparison
    int is_same = (arr_ptr == arr); // 1
    int is_diff = ((arr_ptr + 1) != arr_ptr); // 1

    // Result: val(25) + a(2) + b(1) + arr[1](5) + elem2(30) - 21 = 42
    int res = val + a + b + arr[1] + elem2 - 21;
    printf("val=%d a=%d b=%d arr1=%d elem2=%d same=%d diff=%d res=%d\n",
           val, a, b, arr[1], elem2, is_same, is_diff, res);
    return res;
}
