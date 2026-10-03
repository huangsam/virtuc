#include <stdio.h>

int binary_search(int* arr, int n, int target) {
    int low = 0;
    int high = n - 1;

    while (low <= high) {
        int mid = low + (high - low) / 2;
        if (arr[mid] == target) {
            return mid;
        }
        if (arr[mid] < target) {
            low = mid + 1;
        } else {
            high = mid - 1;
        }
    }

    return -1; // Target not found
}

int main() {
    int arr[7];
    arr[0] = 2;
    arr[1] = 5;
    arr[2] = 8;
    arr[3] = 12;
    arr[4] = 16;
    arr[5] = 23;
    arr[6] = 38;

    int idx_found = binary_search(arr, 7, 23); // expected 5
    int idx_missing = binary_search(arr, 7, 99); // expected -1

    printf("found=%d missing=%d\n", idx_found, idx_missing);

    if (idx_found == 5 && idx_missing == -1) {
        return 42;
    }
    return 0;
}
