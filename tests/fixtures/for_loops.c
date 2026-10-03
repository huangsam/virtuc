int main() {
    int sum = 0;

    // 1. Omitted init
    int i = 0;
    for (; i < 3; i++) {
        sum += 1;
    }

    // 2. Omitted update
    int j = 0;
    for (; j < 3; ) {
        sum += 1;
        j++;
    }

    // 3. Omitted cond
    for (int k = 0; ; k++) {
        sum += 1;
        if (k == 2) {
            break;
        }
    }

    // 4. All clauses omitted
    for (;;) {
        sum += 33;
        break;
    }

    // 3 + 3 + 3 + 33 = 42
    return sum;
}
